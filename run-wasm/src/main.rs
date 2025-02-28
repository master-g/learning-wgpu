fn main() {
	cargo_run_wasm::run_wasm_cli_with_css(
		r#"
    body, div, canvas { margin: 0px; padding: 0px; }
    body {
        display: flex;
        justify-content: center;
        align-items: center;
        background: linear-gradient(135deg,
          white 0%,
          white 49%,
          black 49%,
          black 51%,
          white 51%,
          white 100%) repeat;
        background-size: 20px 20px;
        width: 100vw;
        height: 100vh;
    }
    canvas {
        display: block;
        width: 100%;
        height: 100%;
        background-color: #454545;
    }
    #wgpu-app-container {
        width: 50vw;
        height: 50vw;
        min-width: 375px;
        min-height: 375px;
    }
    "#,
	);
}
