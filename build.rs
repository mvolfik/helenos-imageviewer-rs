use std::{env, path::PathBuf};

fn main() {
    let hel_dirname = match env::var("TARGET").unwrap().split_once('-').map(|x| x.0) {
        Some("i686") => "ia32dyn",
        Some("x86_64") => "amd64dyn",
        x => {
            println!("cargo::error=Unsupported HelenOS target {x:?}");
            return;
        }
    };

    let helenos_path = format!("../helenos/{hel_dirname}/export-dev/include");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let libextern_file = out_path.join("extern.c");

    println!("cargo::rustc-link-arg=-l:libstartfiles.a");
    for lib in [
        "ui", "gfx", "gfxfont", "riff", "memgfx", "display", "console", "ipcgfx", "congfx",
        "pixconv",
    ] {
        println!("cargo::rustc-link-lib={lib}");
    }

    let include_flags = ["ui", "display", "gfx", "c", "input", "console", "output"]
        .map(|lib| format!("-I{}/lib{}", helenos_path, lib));

    let builder = bindgen::Builder::default()
        .header("wrapper.h")
        .sort_semantically(true)
        .allowlist_function(
            "ui_.*|image_(create|set_rect|ctl)|\
            gfx_(bitmap_(create|params_init|get_alloc)|rect_rtranslate)|\
            pixelmap_(put_pixel)|rgba_to_pix",
        )
        .allowlist_var("UI_.*")
        .blocklist_function("ui_create_cons")
        .blocklist_type("sysarg_t|errno_t|console_ctrl_t")
        .bitfield_enum("ui_wdecor_style_t")
        .impl_debug(true)
        .wrap_static_fns(true)
        .wrap_static_fns_path(&libextern_file)
        .clang_args(&include_flags)
        .clang_arg("-nostdinc")
        .clang_arg(format!(
            "-I{}/.local/share/HelenOS/cross/lib/gcc/i686-helenos/14.2.0/include",
            env::var("HOME").expect("HOME env var not set")
        ))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
    let bindings = builder.generate().expect("Unable to generate bindings");

    let mut libextern_build = cc::Build::new();
    libextern_build
        .file(&libextern_file)
        .include(env::var("CARGO_MANIFEST_DIR").unwrap());
    for flag in include_flags {
        libextern_build.flag(&flag);
    }
    libextern_build.compile("extern");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
