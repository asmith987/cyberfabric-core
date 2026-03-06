use std::sync::Arc;

use modkit_odata::ODataQuery;
use modkit_security::AccessScope;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::models::NewChat;

use crate::domain::repos::{
    InsertAssistantMessageParams, InsertUserMessageParams, MessageRepository as MessageRepoTrait,
};
use crate::domain::service::test_helpers::{
    inmem_db, mock_db_provider, mock_enforcer, mock_model_resolver, mock_thread_summary_repo,
    test_security_ctx,
};
use crate::infra::db::repo::chat_repo::ChatRepository as OrmChatRepository;
use crate::infra::db::repo::message_repo::MessageRepository as OrmMessageRepository;

use super::MessageService;
use crate::domain::service::ChatService;

// ── Test Helpers ──

fn limit_cfg() -> modkit_db::odata::LimitCfg {
    modkit_db::odata::LimitCfg {
        default: 20,
        max: 100,
    }
}

fn build_chat_service(
    db_provider: Arc<crate::domain::service::DbProvider>,
    chat_repo: Arc<OrmChatRepository>,
) -> ChatService<OrmChatRepository> {
    ChatService::new(
        db_provider,
        chat_repo,
        mock_thread_summary_repo(),
        mock_enforcer(),
        mock_model_resolver(),
    )
}

fn build_message_service(
    db_provider: Arc<crate::domain::service::DbProvider>,
    chat_repo: Arc<OrmChatRepository>,
) -> MessageService<OrmMessageRepository, OrmChatRepository> {
    let message_repo = Arc::new(OrmMessageRepository::new(limit_cfg()));
    MessageService::new(db_provider, message_repo, chat_repo, mock_enforcer())
}

// ── Tests ──

#[tokio::test]
async fn list_messages_empty_chat() {
    let db = inmem_db().await;
    let db_provider = mock_db_provider(db);
    let chat_repo = Arc::new(OrmChatRepository::new(limit_cfg()));

    let chat_svc = build_chat_service(Arc::clone(&db_provider), Arc::clone(&chat_repo));
    let msg_svc = build_message_service(Arc::clone(&db_provider), Arc::clone(&chat_repo));

    let tenant_id = Uuid::new_v4();
    let ctx = test_security_ctx(tenant_id);

    let chat = chat_svc
        .create_chat(
            &ctx,
            NewChat {
                model: None,
                title: Some("Empty chat".to_owned()),
                is_temporary: false,
            },
        )
        .await
        .expect("create_chat failed");

    let page = msg_svc
        .list_messages(&ctx, chat.id, &ODataQuery::default())
        .await
        .expect("list_messages failed");

    assert!(page.items.is_empty(), "Expected no messages in new chat");
}

#[tokio::test]
async fn list_messages_returns_messages_chronologically() {
    let db = inmem_db().await;
    let db_provider = mock_db_provider(db);
    let chat_repo = Arc::new(OrmChatRepository::new(limit_cfg()));

    let chat_svc = build_chat_service(Arc::clone(&db_provider), Arc::clone(&chat_repo));
    let msg_svc = build_message_service(Arc::clone(&db_provider), Arc::clone(&chat_repo));

    let tenant_id = Uuid::new_v4();
    let ctx = test_security_ctx(tenant_id);

    let chat = chat_svc
        .create_chat(
            &ctx,
            NewChat {
                model: None,
                title: Some("With messages".to_owned()),
                is_temporary: false,
            },
        )
        .await
        .expect("create_chat failed");

    // Insert messages via the repo directly using tenant-scoped access
    let scope = AccessScope::for_tenant(tenant_id);
    let conn = db_provider.conn().expect("conn failed");
    let message_repo = OrmMessageRepository::new(limit_cfg());

    let request_id = Uuid::new_v4();

    message_repo
        .insert_user_message(
            &conn,
            &scope,
            InsertUserMessageParams {
                id: Uuid::now_v7(),
                tenant_id,
                chat_id: chat.id,
                request_id,
                content: "Hello".to_owned(),
            },
        )
        .await
        .expect("insert_user_message failed");

    message_repo
        .insert_assistant_message(
            &conn,
            &scope,
            InsertAssistantMessageParams {
                id: Uuid::now_v7(),
                tenant_id,
                chat_id: chat.id,
                request_id,
                content: "Hi there!".to_owned(),
                input_tokens: Some(10),
                output_tokens: Some(20),
                model: Some("gpt-5.2".to_owned()),
                provider_response_id: None,
            },
        )
        .await
        .expect("insert_assistant_message failed");

    let page = msg_svc
        .list_messages(&ctx, chat.id, &ODataQuery::default())
        .await
        .expect("list_messages failed");

    assert_eq!(page.items.len(), 2, "Expected 2 messages");
    assert_eq!(page.items[0].role, "user", "First message should be user");
    assert_eq!(
        page.items[1].role, "assistant",
        "Second message should be assistant"
    );
    assert!(
        page.items[0].created_at <= page.items[1].created_at,
        "Messages should be in chronological order"
    );
}

#[tokio::test]
async fn list_messages_chat_not_found() {
    let db = inmem_db().await;
    let db_provider = mock_db_provider(db);
    let chat_repo = Arc::new(OrmChatRepository::new(limit_cfg()));

    let msg_svc = build_message_service(db_provider, chat_repo);

    let ctx = test_security_ctx(Uuid::new_v4());
    let random_chat_id = Uuid::new_v4();

    let result = msg_svc
        .list_messages(&ctx, random_chat_id, &ODataQuery::default())
        .await;

    assert!(result.is_err(), "Expected error for non-existent chat");
    assert!(
        matches!(result.unwrap_err(), DomainError::ChatNotFound { .. }),
        "Expected ChatNotFound"
    );
}

#[tokio::test]
async fn list_messages_cross_tenant_returns_not_found() {
    let db = inmem_db().await;
    let db_provider = mock_db_provider(db);
    let chat_repo = Arc::new(OrmChatRepository::new(limit_cfg()));

    let chat_svc = build_chat_service(Arc::clone(&db_provider), Arc::clone(&chat_repo));
    let msg_svc = build_message_service(Arc::clone(&db_provider), Arc::clone(&chat_repo));

    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();
    let ctx_a = test_security_ctx(tenant_a);
    let ctx_b = test_security_ctx(tenant_b);

    // Tenant A creates a chat
    let chat = chat_svc
        .create_chat(
            &ctx_a,
            NewChat {
                model: None,
                title: Some("Tenant A chat".to_owned()),
                is_temporary: false,
            },
        )
        .await
        .expect("create_chat failed");

    // Tenant B tries to list messages in Tenant A's chat
    let result = msg_svc
        .list_messages(&ctx_b, chat.id, &ODataQuery::default())
        .await;

    assert!(result.is_err(), "Cross-tenant list must fail");
    assert!(
        matches!(result.unwrap_err(), DomainError::ChatNotFound { .. }),
        "Expected ChatNotFound for cross-tenant access"
    );
}

// ── Pagination Tests ──

/// Insert N user+assistant message pairs into a chat via the repo directly.
async fn insert_message_pairs(
    db_provider: &Arc<crate::domain::service::DbProvider>,
    tenant_id: Uuid,
    chat_id: Uuid,
    count: usize,
) {
    let scope = AccessScope::for_tenant(tenant_id);
    let conn = db_provider.conn().expect("conn failed");
    let message_repo = OrmMessageRepository::new(limit_cfg());

    for _ in 0..count {
        let request_id = Uuid::new_v4();

        message_repo
            .insert_user_message(
                &conn,
                &scope,
                InsertUserMessageParams {
                    id: Uuid::now_v7(),
                    tenant_id,
                    chat_id,
                    request_id,
                    content: "Hello".to_owned(),
                },
            )
            .await
            .expect("insert_user_message failed");

        message_repo
            .insert_assistant_message(
                &conn,
                &scope,
                InsertAssistantMessageParams {
                    id: Uuid::now_v7(),
                    tenant_id,
                    chat_id,
                    request_id,
                    content: "Hi there!".to_owned(),
                    input_tokens: Some(10),
                    output_tokens: Some(20),
                    model: Some("gpt-5.2".to_owned()),
                    provider_response_id: None,
                },
            )
            .await
            .expect("insert_assistant_message failed");
    }
}

#[tokio::test]
async fn list_messages_pagination_forward_cursor() {
    let db = inmem_db().await;
    let db_provider = mock_db_provider(db);
    let chat_repo = Arc::new(OrmChatRepository::new(limit_cfg()));

    let chat_svc = build_chat_service(Arc::clone(&db_provider), Arc::clone(&chat_repo));
    let msg_svc = build_message_service(Arc::clone(&db_provider), Arc::clone(&chat_repo));

    let tenant_id = Uuid::new_v4();
    let ctx = test_security_ctx(tenant_id);

    let chat = chat_svc
        .create_chat(
            &ctx,
            NewChat {
                model: None,
                title: Some("Pagination chat".to_owned()),
                is_temporary: false,
            },
        )
        .await
        .expect("create_chat failed");

    // Insert 5 pairs = 10 messages
    insert_message_pairs(&db_provider, tenant_id, chat.id, 5).await;

    // Page 1: request 3 items
    let query = ODataQuery::new().with_limit(3);
    let page1 = msg_svc
        .list_messages(&ctx, chat.id, &query)
        .await
        .expect("list_messages page 1 failed");

    assert_eq!(page1.items.len(), 3, "Page 1 should have 3 items");
    assert!(
        page1.page_info.next_cursor.is_some(),
        "Page 1 must have next_cursor (7 more messages remain)"
    );
    assert!(
        page1.page_info.prev_cursor.is_none(),
        "Page 1 must not have prev_cursor (first page)"
    );

    // Page 2: use next_cursor
    let cursor = modkit_odata::CursorV1::decode(page1.page_info.next_cursor.as_ref().unwrap())
        .expect("decode cursor failed");
    let query2 = ODataQuery::new().with_limit(3).with_cursor(cursor);
    let page2 = msg_svc
        .list_messages(&ctx, chat.id, &query2)
        .await
        .expect("list_messages page 2 failed");

    assert_eq!(page2.items.len(), 3, "Page 2 should have 3 items");
    assert!(
        page2.page_info.next_cursor.is_some(),
        "Page 2 must have next_cursor (4 more messages remain)"
    );

    // Continue until exhausted, collecting all IDs
    let mut all_ids: Vec<Uuid> = page1
        .items
        .iter()
        .chain(page2.items.iter())
        .map(|m| m.id)
        .collect();

    let mut current_page = page2;
    while let Some(ref next) = current_page.page_info.next_cursor {
        let cursor = modkit_odata::CursorV1::decode(next).expect("decode cursor failed");
        let q = ODataQuery::new().with_limit(3).with_cursor(cursor);
        current_page = msg_svc
            .list_messages(&ctx, chat.id, &q)
            .await
            .expect("list_messages next page failed");
        all_ids.extend(current_page.items.iter().map(|m| m.id));
    }

    assert_eq!(
        all_ids.len(),
        10,
        "Total messages across all pages should be 10"
    );
    let unique_count = {
        let mut sorted = all_ids.clone();
        sorted.sort();
        sorted.dedup();
        sorted.len()
    };
    assert_eq!(unique_count, 10, "All message IDs must be unique");
}

#[tokio::test]
async fn list_messages_pagination_no_cursor_when_all_fit() {
    let db = inmem_db().await;
    let db_provider = mock_db_provider(db);
    let chat_repo = Arc::new(OrmChatRepository::new(limit_cfg()));

    let chat_svc = build_chat_service(Arc::clone(&db_provider), Arc::clone(&chat_repo));
    let msg_svc = build_message_service(Arc::clone(&db_provider), Arc::clone(&chat_repo));

    let tenant_id = Uuid::new_v4();
    let ctx = test_security_ctx(tenant_id);

    let chat = chat_svc
        .create_chat(
            &ctx,
            NewChat {
                model: None,
                title: Some("Small chat".to_owned()),
                is_temporary: false,
            },
        )
        .await
        .expect("create_chat failed");

    // Insert 2 pairs = 4 messages, request page of 20
    insert_message_pairs(&db_provider, tenant_id, chat.id, 2).await;

    let query = ODataQuery::new().with_limit(20);
    let page = msg_svc
        .list_messages(&ctx, chat.id, &query)
        .await
        .expect("list_messages failed");

    assert_eq!(page.items.len(), 4);
    assert!(
        page.page_info.next_cursor.is_none(),
        "No next_cursor when all messages fit in a single page"
    );
    assert!(
        page.page_info.prev_cursor.is_none(),
        "No prev_cursor on the first (and only) page"
    );
}

#[tokio::test]
async fn list_messages_pagination_backward_cursor() {
    let db = inmem_db().await;
    let db_provider = mock_db_provider(db);
    let chat_repo = Arc::new(OrmChatRepository::new(limit_cfg()));

    let chat_svc = build_chat_service(Arc::clone(&db_provider), Arc::clone(&chat_repo));
    let msg_svc = build_message_service(Arc::clone(&db_provider), Arc::clone(&chat_repo));

    let tenant_id = Uuid::new_v4();
    let ctx = test_security_ctx(tenant_id);

    let chat = chat_svc
        .create_chat(
            &ctx,
            NewChat {
                model: None,
                title: Some("Backward pagination chat".to_owned()),
                is_temporary: false,
            },
        )
        .await
        .expect("create_chat failed");

    // Insert 5 pairs = 10 messages
    insert_message_pairs(&db_provider, tenant_id, chat.id, 5).await;

    // Page 1 forward (3 items)
    let query = ODataQuery::new().with_limit(3);
    let page1 = msg_svc
        .list_messages(&ctx, chat.id, &query)
        .await
        .expect("list_messages page 1 failed");
    assert_eq!(page1.items.len(), 3);

    // Page 2 forward
    let cursor = modkit_odata::CursorV1::decode(page1.page_info.next_cursor.as_ref().unwrap())
        .expect("decode cursor failed");
    let query2 = ODataQuery::new().with_limit(3).with_cursor(cursor);
    let page2 = msg_svc
        .list_messages(&ctx, chat.id, &query2)
        .await
        .expect("list_messages page 2 failed");
    assert_eq!(page2.items.len(), 3);
    assert!(
        page2.page_info.prev_cursor.is_some(),
        "Page 2 must have prev_cursor"
    );

    // Navigate backward from page 2
    let prev = modkit_odata::CursorV1::decode(page2.page_info.prev_cursor.as_ref().unwrap())
        .expect("decode prev cursor failed");
    let query_back = ODataQuery::new().with_limit(3).with_cursor(prev);
    let page_back = msg_svc
        .list_messages(&ctx, chat.id, &query_back)
        .await
        .expect("list_messages backward failed");

    assert_eq!(
        page_back.items.len(),
        page1.items.len(),
        "Backward page should have same count as page 1"
    );
    let back_ids: Vec<Uuid> = page_back.items.iter().map(|m| m.id).collect();
    let page1_ids: Vec<Uuid> = page1.items.iter().map(|m| m.id).collect();
    assert_eq!(
        back_ids, page1_ids,
        "Backward navigation must return to page 1 items"
    );
}
