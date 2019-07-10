use std::collections::HashMap;
use yaml_rust::Yaml;
use std::process::Command;
use terminal_size::{Width, Height, terminal_size};
use std::io::{self, Write};
use std::env;

use crate::template;

const RECUSRION_COUNT_VAR_NAME : &str = "JOAT_RECURSION_COUNT";
const COLUMNS_ENV_VAR_NAME : &str = "COLUMNS";


fn get_terminal_width() -> u16 {
    match env::var(COLUMNS_ENV_VAR_NAME) {
        Ok(c) => return c.parse().expect("Coulumns should be u16 integers"),
        Err(_) => (),
    }
    let size = terminal_size();
    if let Some((Width(w), Height(_h))) = size {
        return w;
    }
    return 80;
}

fn check_recursion_count(yaml: &Yaml) -> i64 {
    let max_recursion_count;
    if yaml["max_recursion_count"].is_badvalue() {
        max_recursion_count = 100;
    } else {
        max_recursion_count = yaml["max_recursion_count"].clone().into_i64()
            .expect("Max recursion should be an integer");
    }
    let recursion_count: i64 = match env::var(RECUSRION_COUNT_VAR_NAME) {
        Ok(count) => count.parse()
            .expect(&format!("{} should be of type i64", RECUSRION_COUNT_VAR_NAME)),
        Err(_e) => 0,
    };
    if recursion_count > max_recursion_count {
        println!("Max recursion count ({:?}) reached", max_recursion_count);
        println!("Check for infinite loops in your yaml or increase max_recursion_count config");
        ::std::process::exit(1);
    }
    return recursion_count + 1;
}

pub fn execute_script(context: HashMap<String, HashMap<String,String>>, subcmd_yaml: &Yaml, yaml: &Yaml,) {
    let script_string = subcmd_yaml["script"].clone().into_string()
        .expect("Could not convert script to string");
    let script = template::get_compiled_template_str_with_context(
        &script_string,
        &context)
        .expect(format!("Could not parse script template {:?}", script_string).as_str());
    let columns = get_terminal_width();
    let recursion_count = check_recursion_count(yaml);
    let output = Command::new("bash")
            .arg("-c")
            .arg(script)
            .env(COLUMNS_ENV_VAR_NAME, columns.to_string())
            .env(RECUSRION_COUNT_VAR_NAME, recursion_count.to_string())
            .output()
            .expect("failed to execute script");
    if !context["args"].contains_key("quiet") {
        io::stdout().write_all(&output.stdout).unwrap();
        io::stderr().write_all(&output.stderr).unwrap();
    }

    ::std::process::exit(output.status.code().expect("Unknown exit code"));
}