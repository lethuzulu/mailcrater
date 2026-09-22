use yew::prelude::*;

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
                    Ok(response) => match response.text().await {
                        Ok(text) => version.set(VersionState::Loaded(text)),
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
        </main>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
