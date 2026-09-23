use serde::Deserialize;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Deserialize, Clone, PartialEq)]
pub struct MessageSummary {
    pub id: String,
    pub received_at: String,
    pub from_addr: String,
    pub to_addrs: Vec<String>,
    pub subject: Option<String>,
}

#[derive(Deserialize, Clone, PartialEq)]
struct MessageListResponse {
    messages: Vec<MessageSummary>,
    total: i64,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: String,
}

#[derive(Clone, PartialEq)]
enum ListState {
    Loading,
    Loaded(MessageListResponse),
    Failed(String),
}

async fn fetch_messages(search: String) -> ListState {
    let url = match search.is_empty() {
        true => "/api/messages".to_string(),
        false => {
            let encoded: String = js_sys::encode_uri_component(&search).into();
            format!("/api/messages?search={encoded}")
        }
    };

    let response = match gloo_net::http::Request::get(&url).send().await {
        Ok(response) => response,
        Err(e) => return ListState::Failed(e.to_string()),
    };

    match response.ok() {
        true => match response.json::<MessageListResponse>().await {
            Ok(body) => ListState::Loaded(body),
            Err(e) => ListState::Failed(e.to_string()),
        },
        false => match response.json::<ErrorBody>().await {
            Ok(body) => ListState::Failed(body.error),
            Err(_) => ListState::Failed(format!("request failed: {}", response.status())),
        },
    }
}

fn run_fetch(state: UseStateHandle<ListState>, search: String) {
    state.set(ListState::Loading);
    yew::platform::spawn_local(async move {
        let result = fetch_messages(search).await;
        state.set(result);
    });
}

async fn delete_all() -> Result<(), String> {
    let response = match gloo_net::http::Request::delete("/api/messages").send().await {
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
pub struct MessageListProps {
    pub on_select: Callback<String>,
    pub selected_id: Option<String>,
    pub refresh_signal: u32,
}

#[function_component(MessageList)]
pub fn message_list(props: &MessageListProps) -> Html {
    let state = use_state(|| ListState::Loading);
    let search = use_state(String::new);

    {
        let state = state.clone();
        let search = search.clone();
        use_effect_with(props.refresh_signal, move |_| {
            run_fetch(state, (*search).clone());
        });
    }

    let on_refresh = {
        let state = state.clone();
        let search = search.clone();
        Callback::from(move |_: MouseEvent| {
            run_fetch(state.clone(), (*search).clone());
        })
    };

    let on_search_input = {
        let state = state.clone();
        let search = search.clone();
        Callback::from(move |e: InputEvent| {
            let value = match e.target_dyn_into::<HtmlInputElement>() {
                Some(input) => input.value(),
                None => return,
            };
            search.set(value.clone());
            run_fetch(state.clone(), value);
        })
    };

    let on_delete_all = {
        let state = state.clone();
        let search = search.clone();
        Callback::from(move |_: MouseEvent| {
            let state = state.clone();
            let search = search.clone();
            yew::platform::spawn_local(async move {
                match delete_all().await {
                    Ok(()) => run_fetch(state, (*search).clone()),
                    Err(e) => state.set(ListState::Failed(e)),
                }
            });
        })
    };

    html! {
        <aside class="sidebar">
            <div class="sidebar-controls">
                <input
                    type="text"
                    class="search-input"
                    placeholder="Search from, to, subject..."
                    value={(*search).clone()}
                    oninput={on_search_input}
                />
                <button onclick={on_refresh}>{ "Refresh" }</button>
                <button onclick={on_delete_all}>{ delete_all_label(&state) }</button>
            </div>
            { render_body(&state, &props.on_select, &props.selected_id) }
        </aside>
    }
}

fn delete_all_label(state: &ListState) -> String {
    match state {
        ListState::Loaded(response) => format!("Delete all ({})", response.total),
        _ => "Delete all".to_string(),
    }
}

fn render_body(
    state: &ListState,
    on_select: &Callback<String>,
    selected_id: &Option<String>,
) -> Html {
    match state {
        ListState::Loading => html! { <p class="sidebar-message">{ "Loading..." }</p> },
        ListState::Failed(error) => {
            html! { <p class="sidebar-message error">{ format!("Error: {error}") }</p> }
        }
        ListState::Loaded(response) => match response.messages.is_empty() {
            true => html! { <p class="sidebar-message">{ "The inbox is empty." }</p> },
            false => html! {
                <>
                    <p class="sidebar-total">{ format!("{} total", response.total) }</p>
                    <ul class="message-list">
                        { for response.messages.iter().map(|m| render_row(m, on_select, selected_id)) }
                    </ul>
                </>
            },
        },
    }
}

fn render_row(
    message: &MessageSummary,
    on_select: &Callback<String>,
    selected_id: &Option<String>,
) -> Html {
    let subject = match &message.subject {
        Some(subject) => subject.clone(),
        None => String::new(),
    };

    let id = message.id.clone();
    let on_select = on_select.clone();
    let onclick = Callback::from(move |_: MouseEvent| on_select.emit(id.clone()));

    let is_selected = selected_id.as_deref() == Some(message.id.as_str());
    let mut classes = classes!("message-row");
    if is_selected {
        classes.push("selected");
    }

    html! {
        <li key={message.id.clone()} class={classes} onclick={onclick}>
            <span class="message-icon">{ "\u{2709}" }</span>
            <div class="message-row-main">
                <div class="message-row-top">
                    <span class="message-from">{ &message.from_addr }</span>
                    <span class="message-received">{ &message.received_at }</span>
                </div>
                <div class="message-subject">{ subject }</div>
                <div class="message-to">{ format!("To: {}", message.to_addrs.join(", ")) }</div>
            </div>
        </li>
    }
}
