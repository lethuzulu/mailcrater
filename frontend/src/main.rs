mod message_list;

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

    html! {
        <main>
            <h1>{ "MailCrater" }</h1>
            <p>{ status }</p>
            <MessageList />
        </main>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
