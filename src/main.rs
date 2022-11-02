#[cfg(target_arch = "wasm32")]
fn main() {
    use minesweeper::app::{App, Props};
    use minesweeper::config::Config;

    yew::start_app_with_props::<App>(Props {
        config: Config::from_build_env(),
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("minesweeper renders in a browser: run `trunk serve --open`");
}
