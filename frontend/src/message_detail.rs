use serde::Deserialize;
use yew::prelude::*;

#[derive(Deserialize, Clone, PartialEq)]
struct AttachmentMeta {
    id: String,
    filename: Option<String>,
    content_type: Option<String>,
    size: i64,
}

#[derive(Deserialize, Clone, PartialEq)]
struct MessageDetail {
    id: String,
    received_at: String,
    from_addr: String,
    to_addrs: Vec<String>,
    cc_addrs: Vec<String>,
    subject: Option<String>,
    body_text: Option<String>,
    body_html: Option<String>,
    attachments: Vec<AttachmentMeta>,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: String,
}

#[derive(Clone, PartialEq)]
enum DetailState {
    Loading,
    Loaded(MessageDetail),
    Failed(String),
}


#[derive(Clone, Copy, PartialEq)]
enum BodyTab {
    Html,
    Plain,
}

async fn fetch_detail(id: String) -> DetailState {
    let url = format!("/api/messages/{id}");

    let response = match gloo_net::http::Request::get(&url).send().await {
        Ok(response) => response,
        Err(e) => return DetailState::Failed(e.to_string()),
    };

    match response.ok() {
        true => match response.json::<MessageDetail>().await {
            Ok(body) => DetailState::Loaded(body),
            Err(e) => DetailState::Failed(e.to_string()),
        },
        false => match response.json::<ErrorBody>().await {
            Ok(body) => DetailState::Failed(body.error),
            Err(_) => DetailState::Failed(format!("request failed: {}", response.status())),
        },
    }
}

async fn delete_message(id: String) -> Result<(), String> {
    let url = format!("/api/messages/{id}");

    let response = match gloo_net::http::Request::delete(&url).send().await {
        Ok(response) => response,
        Err(e) => return Err(e.to_string()),
    };

    match response.ok() {
        true => Ok(()),
        false => match response.json::<ErrorBody>().await {
            Ok(body) => Err(body.error),
            Err(_) => Err(format!("request failed: {}", response.status())),
        },
    }
}

#[derive(Properties, PartialEq)]
pub struct MessageDetailProps {
    pub id: String,
    pub on_deleted: Callback<()>,
}

#[function_component(MessageDetailView)]
pub fn message_detail_view(props: &MessageDetailProps) -> Html {
    let state = use_state(|| DetailState::Loading);
    let tab = use_state(|| BodyTab::Html);

    {
        let state = state.clone();
        let tab = tab.clone();
        let id = props.id.clone();
        use_effect_with(id.clone(), move |_| {
            state.set(DetailState::Loading);
            let state = state.clone();
            let tab = tab.clone();
            yew::platform::spawn_local(async move {
                let result = fetch_detail(id).await;
                if let DetailState::Loaded(detail) = &result {
                    let default_tab = match detail.body_html.is_some() {
                        true => BodyTab::Html,
                        false => BodyTab::Plain,
                    };
                    tab.set(default_tab);
                }
                state.set(result);
            });
        });
    }

    let on_delete = {
        let state = state.clone();
        let id = props.id.clone();
        let on_deleted = props.on_deleted.clone();
        Callback::from(move |_: MouseEvent| {
            let state = state.clone();
            let id = id.clone();
            let on_deleted = on_deleted.clone();
            yew::platform::spawn_local(async move {
                match delete_message(id).await {
                    Ok(()) => on_deleted.emit(()),
                    Err(e) => state.set(DetailState::Failed(e)),
                }
            });
        })
    };

    match &*state {
        DetailState::Loading => html! { <p>{ "Loading message..." }</p> },
        DetailState::Failed(error) => html! { <p>{ format!("Error: {error}") }</p> },
        DetailState::Loaded(detail) => render_detail(detail, &tab, &on_delete),
    }
}

fn render_detail(
    detail: &MessageDetail,
    tab: &UseStateHandle<BodyTab>,
    on_delete: &Callback<MouseEvent>,
) -> Html {
    let raw_url = format!("/api/messages/{}/raw", detail.id);

    let on_html_tab = {
        let tab = tab.clone();
        Callback::from(move |_: MouseEvent| tab.set(BodyTab::Html))
    };
    let on_plain_tab = {
        let tab = tab.clone();
        Callback::from(move |_: MouseEvent| tab.set(BodyTab::Plain))
    };

    let mut html_tab_classes = classes!("tab");
    let mut plain_tab_classes = classes!("tab");
    match **tab {
        BodyTab::Html => html_tab_classes.push("active"),
        BodyTab::Plain => plain_tab_classes.push("active"),
    }

    let body = match **tab {
        BodyTab::Html => match &detail.body_html {
            Some(html_body) => {
                html! { <iframe class="body-frame" sandbox="" srcdoc={html_body.clone()}></iframe> }
            }
            None => html! { <p class="body-empty">{ "No HTML body." }</p> },
        },
        BodyTab::Plain => match &detail.body_text {
            Some(text) => html! { <pre class="body-plain">{ text }</pre> },
            None => html! { <p class="body-empty">{ "No plain text body." }</p> },
        },
    };

    html! {
        <div class="message-detail">
            <table class="message-meta">
                <tbody>
                    <tr><td>{ "From" }</td><td>{ &detail.from_addr }</td></tr>
                    <tr><td>{ "To" }</td><td>{ detail.to_addrs.join(", ") }</td></tr>
                    { render_cc(detail) }
                    <tr><td>{ "Subject" }</td><td>{ detail.subject.clone().unwrap_or_default() }</td></tr>
                    <tr><td>{ "Received" }</td><td>{ &detail.received_at }</td></tr>
                </tbody>
            </table>
            <div class="tab-bar">
                <button class={html_tab_classes} onclick={on_html_tab}>{ "HTML" }</button>
                <button class={plain_tab_classes} onclick={on_plain_tab}>{ "Plain" }</button>
                <a class="tab" href={raw_url} target="_blank">{ "Raw" }</a>
                <button class="delete-button" onclick={on_delete.clone()}>{ "Delete" }</button>
            </div>
            <div class="body-container">
                { body }
            </div>
            { render_attachments(detail) }
        </div>
    }
}

fn render_cc(detail: &MessageDetail) -> Html {
    match detail.cc_addrs.is_empty() {
        true => html! {},
        false => html! { <tr><td>{ "Cc" }</td><td>{ detail.cc_addrs.join(", ") }</td></tr> },
    }
}

fn render_attachments(detail: &MessageDetail) -> Html {
    match detail.attachments.is_empty() {
        true => html! {},
        false => html! {
            <div class="attachments">
                <h3>{ "Attachments" }</h3>
                <ul>
                    { for detail.attachments.iter().map(|a| render_attachment(&detail.id, a)) }
                </ul>
            </div>
        },
    }
}

fn render_attachment(message_id: &str, attachment: &AttachmentMeta) -> Html {
    let url = format!("/api/messages/{message_id}/attachments/{}", attachment.id);
    let filename = match &attachment.filename {
        Some(name) => name.clone(),
        None => "attachment".to_string(),
    };
    let content_type = match &attachment.content_type {
        Some(content_type) => content_type.clone(),
        None => "unknown type".to_string(),
    };

    html! {
        <li key={attachment.id.clone()}>
            <a href={url}>{ format!("{filename} ({content_type}, {} bytes)", attachment.size) }</a>
        </li>
    }
}
