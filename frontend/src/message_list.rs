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

#[function_component(MessageList)]
pub fn message_list() -> Html {
    let state = use_state(|| ListState::Loading);
    let search = use_state(String::new);

    {
        let state = state.clone();
        use_effect_with((), move |_| {
            run_fetch(state, String::new());
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

    html! {
        <div>
            <div>
                <input
                    type="text"
                    placeholder="Search from, to, subject..."
                    value={(*search).clone()}
                    oninput={on_search_input}
                />
                <button onclick={on_refresh}>{ "Refresh" }</button>
            </div>
            { render_body(&state) }
        </div>
    }
}

fn render_body(state: &ListState) -> Html {
    match state {
        ListState::Loading => html! { <p>{ "Loading..." }</p> },
        ListState::Failed(error) => html! { <p>{ format!("Error: {error}") }</p> },
        ListState::Loaded(response) => match response.messages.is_empty() {
            true => html! { <p>{ "No messages." }</p> },
            false => html! {
                <>
                    <p>{ format!("{} total", response.total) }</p>
                    <table>
                        <thead>
                            <tr>
                                <th>{ "From" }</th>
                                <th>{ "Subject" }</th>
                                <th>{ "Received" }</th>
                            </tr>
                        </thead>
                        <tbody>
                            { for response.messages.iter().map(render_row) }
                        </tbody>
                    </table>
                </>
            },
        },
    }
}

fn render_row(message: &MessageSummary) -> Html {
    let subject = match &message.subject {
        Some(subject) => subject.clone(),
        None => String::new(),
    };

    html! {
        <tr key={message.id.clone()}>
            <td>{ &message.from_addr }</td>
            <td>{ subject }</td>
            <td>{ &message.received_at }</td>
        </tr>
    }
}
