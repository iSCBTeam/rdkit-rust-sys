use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use itertools::Itertools;
use pkg_config::Config;
use pkg_config::Library;
use walkdir::WalkDir;

fn pkgconfig_lib_inc_args<'s>(lib: &'s Library) -> impl IntoIterator<Item = String> + 's {
	lib.defines.iter().map(|(k, v)|
		match v {
			None => format!("-D{}", k),
			Some(v) => format!("-D{}={}", k, v),
		}
	).chain(lib.include_paths.iter().map(|path|
		format!("-I{}", path.to_string_lossy()))
	)
}

fn get_library_name_and_kind(mut name: &str) -> (&str, bool) {
	if let Some(n) = name.strip_prefix("lib") {
		name = n;
	}
	if let Some(n) = name.strip_suffix(|c| char::is_numeric(c) || c == '.') {
		name = n;
	}

	if let Some(name) = name.strip_suffix(".dll.a") {
		(name, false)
	} else if let Some(name) = name.strip_suffix(".a") {
		(name, false)
	} else if let Some(name) = name.strip_suffix(".lib") {
		(name, false)
	} else if let Some(name) = name.strip_suffix(".dll") {
		(name, true)
	} else if let Some(name) = name.strip_suffix(".dylib") {
		(name, true)
	} else if let Some(name) = name.strip_suffix(".so") {
		(name, true)
	} else {
		(name, true)
	}
}

fn main() {
	let rdkit_cffi_ll_prefix = "../rdkit-cffi-ll/prefix";
	let out_dir = env::var_os("OUT_DIR").unwrap();

	let lib = Config::new()
		.env_metadata(true)
		.cargo_metadata(false)
		.arg(format!("--with-path={}/lib/pkgconfig", rdkit_cffi_ll_prefix))
		.statik(true)
		.probe("rdkit-cffi-ll")
		.expect("rdkit-cffi-ll library not found");
	let lib_inc_args = pkgconfig_lib_inc_args(&lib);

	for args in &lib.ld_args {
		for arg in args {
			println!("cargo:rustc-link-arg={}", arg);
		}
	}

	lib.link_paths.iter().map(|path|
		path.to_string_lossy()
	).chain(lib.link_files.iter().filter_map(|path|
		path.parent().map(|p| p.to_string_lossy()))
	).unique().for_each(|p|
		println!("cargo:rustc-link-search=native={}", p)
	);

	for name in &lib.libs {
		if name == "rdkit-cffi-ll" {
			continue
		}

		println!("cargo:rustc-link-lib={}", name);
	}

	lib.link_files.iter().filter_map(|path|
		path.file_name().map(|filename| {
			let filename = &filename.to_string_lossy();
			let res = get_library_name_and_kind(filename);
			(res.0.to_owned(), res.1)
		})
	).chain(lib.libs.iter().filter_map(|name|
		match name.as_str() {
			// Assume -l libs are dynamic, except for rdkit-cffi-ll
			"rdkit-cffi-ll" => Some((name.to_owned(), false)),
			_ => Some((name.to_owned(), true)),
		})
	).filter(|elem|
		match elem.0.as_str() {
			"boost_iostreams" => false,
			"boost_serialization" => false,
			"boost_system" => false,
			_ => true,
		}
	).chain(
		// Specify these libs explicitely because the pkg-config crate is unable to recognize the versioned name:
		// `warning: File path /usr/lib/libboost_iostreams.so.1.83.0 found in pkg-config file for rdkit-cffi-ll,
		// but could not extract library base name to pass to linker command line`
		[
			("boost_iostreams".to_owned(), false),
			("boost_serialization".to_owned(), false),
			("boost_system".to_owned(), false),
		].into_iter()
	).unique().for_each(|(name, dynamic)|
		if dynamic {
			println!("cargo:rustc-link-lib={}", name);
		} else {
			println!("cargo:rustc-link-lib=static={}", name);
		}
	);

	let wrapper_header_path = Path::new(&out_dir).join("wrapper.h");
	let mut wrapper_header = File::create(&wrapper_header_path).unwrap();

	let headers_path = Path::new(&rdkit_cffi_ll_prefix).join("include");
	let header_paths = WalkDir::new(&headers_path)
		.into_iter()
		.filter_map(|e| e.ok())
		.filter(|e| e.file_type().is_file() && e.file_name().to_str().unwrap().ends_with(".h"))
		.map(|e| e.into_path())
		.sorted_unstable();

	for path in header_paths {
		let path_str = path.to_str().unwrap();
		let rel_path_str = path.strip_prefix(&headers_path).unwrap().to_str().unwrap();

		println!("cargo:rerun-if-changed={}", path_str);
		writeln!(wrapper_header, "#include <{}>", rel_path_str).unwrap();
	}

	wrapper_header.sync_data().unwrap();

	println!("cargo:rerun-if-changed={}", Path::new(&rdkit_cffi_ll_prefix).join("lib").to_str().unwrap());

	let builder = bindgen::Builder::default()
		.clang_args(lib_inc_args)
		.header(wrapper_header_path.to_string_lossy());
	let bindings = builder.generate()
		.expect("Unable to generate bindings");

	let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
	bindings
		.write_to_file(out_path.join("bindings.rs"))
		.expect("Couldn't write bindings!");
}
