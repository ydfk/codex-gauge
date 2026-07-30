fn main() {
    slint_build::compile_with_config(
        "ui/gauge.slint",
        slint_build::CompilerConfiguration::new().with_style("fluent-dark".into()),
    )
    .expect("编译 Slint 界面失败");

    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let mut resource = winresource::WindowsResource::new();
        resource.set("ProductName", "Codex Gauge Native");
        resource.set("FileDescription", "Codex Gauge Native Windows Client");
        resource.set_icon("assets/icon.ico");
        resource.compile().expect("编译 Windows 资源失败");
    }
}
