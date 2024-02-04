use derive_builder::Builder;

#[derive(Builder)]
struct Command {
    executable: String,
    // #[builder(each = "arg")]
    args: Vec<String>,
    // #[builder(each = "env")]
    env: Vec<String>,
    current_dir: Option<String>,
}
