mod message_detail;
mod message_list;

use message_detail::MessageDetailView;
use message_list::MessageList;
use serde::Deserialize;
use yew::prelude::*;

#[derive(Deserialize)]
struct VersionResponse {
    version: String,
}

#[derive(Clone, PartialEq)]
enum VersionState {
    Loading,
    Loaded(String),
    Failed(String),
}

#[function_component(App)]
fn app() -> Html {
    let version = use_state(|| VersionState::Loading);
    let selected_id = use_state(|| None::<String>);
    let refresh_signal = use_state(|| 0u32);

    {
        let version = version.clone();
        use_effect_with((), move |_| {
            yew::platform::spawn_local(async move {
                match gloo_net::http::Request::get("/api/version").send().await {
                    Ok(response) => match response.json::<VersionResponse>().await {
                        Ok(body) => version.set(VersionState::Loaded(body.version)),
                        Err(e) => version.set(VersionState::Failed(e.to_string())),
                    },
                    Err(e) => version.set(VersionState::Failed(e.to_string())),
                }
            });
        });
    }

    let status = match &*version {
        VersionState::Loading => "loading backend version...".to_string(),
        VersionState::Loaded(v) => format!("backend version: {v}"),
        VersionState::Failed(e) => format!("failed to reach backend: {e}"),
    };

    let on_select = {
        let selected_id = selected_id.clone();
        Callback::from(move |id: String| selected_id.set(Some(id)))
    };


    let on_deleted = {
        let selected_id = selected_id.clone();
        let refresh_signal = refresh_signal.clone();
        Callback::from(move |()| {
            selected_id.set(None);
            refresh_signal.set(*refresh_signal + 1);
        })
    };

    let detail = match &*selected_id {
        Some(id) => html! { <MessageDetailView id={id.clone()} on_deleted={on_deleted} /> },
        None => html! { <p class="empty-state">{ "Select a message to view it." }</p> },
    };

    html! {
        <div class="app">
            <header class="app-header">
                <h1><span class="brand-mail">{ "Mail" }</span><span class="brand-crater">{ "Crater" }</span></h1>
                <span class="version-status">{ status }</span>
            </header>
            <div class="app-body">
                <MessageList
                    on_select={on_select}
                    selected_id={(*selected_id).clone()}
                    refresh_signal={*refresh_signal}
                />
                <main class="detail-pane">
                    { detail }
                </main>
            </div>
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
