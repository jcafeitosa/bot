use reqwest::Url;

pub(crate) fn redirect_allowed(previous: &[Url], destination: &Url) -> bool {
    let Some(initial) = previous.first() else {
        return false;
    };

    previous.len() <= 10
        && matches!(initial.scheme(), "http" | "https")
        && initial.scheme() == destination.scheme()
        && initial.host().is_some()
        && initial.host() == destination.host()
        && initial.port_or_known_default() == destination.port_or_known_default()
        && initial.username().is_empty()
        && initial.password().is_none()
        && destination.username().is_empty()
        && destination.password().is_none()
}
