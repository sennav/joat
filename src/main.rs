extern crate clap;
extern crate reqwest;
extern crate serde_json;
extern crate tera;
extern crate yaml_rust;
extern crate serde;
extern crate regex;
extern crate dirs;

use clap::{
    App,
    ArgMatches,
    Shell,
};
use serde_json::value::Value;
use serde_json::Number;
use std::collections::HashMap;
use yaml_rust::Yaml;
use std::process::Command;
use std::io::{self, Write};
use std::{env, fs};
use regex::Regex;
use terminal_size::{Width, Height, terminal_size};
use std::str::FromStr;

mod template;
mod http;
mod yaml;
mod oauth;

const RECUSRION_COUNT_VAR_NAME : &str = "DJOAT_RECURSION_COUNT";
const COLUMNS_ENV_VAR_NAME : &str = "COLUMNS";

fn get_args_context(args: &ArgMatches, subcmd_yaml: &Yaml) -> HashMap<String, String> {
    let mut args_context = HashMap::new();
    for arg in subcmd_yaml["args"].clone().into_iter() {
        for a in arg.into_hash().unwrap().keys() {
            let key = a.clone().into_string().unwrap();
            if args.is_present(&key) {
                let value = args.value_of(&key).unwrap_or("");
                args_context.insert(key, String::from(value));
            }
        }
    }
    return args_context;
}

fn get_vars_context(yaml: &Yaml) -> HashMap<String, String> {
    let mut vars_context = HashMap::new();
    let vars_yaml = &yaml["vars"];
    if !vars_yaml.is_badvalue() {
        let vars_iter = vars_yaml
            .clone()
            .into_hash()
            .expect("Could not convert vars into hash");
        for (key, value) in vars_iter {
            let key_str = key.clone().into_string().expect("Var key should be string");
            let value_str = value.clone().into_string().expect("Var value should be string");
            vars_context.insert(key_str, value_str);
        }
    }
    return vars_context;
}

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

fn execute_script(context: HashMap<String, HashMap<String,String>>, subcmd_yaml: &Yaml, yaml: &Yaml,) {
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

fn get_parsed_yaml_key(key: &str, yaml: &Yaml, error_str: &str, context: &HashMap<String, HashMap<String, String>>) -> String {
    template::get_compiled_template_str_with_context(
        &yaml[key]
            .clone()
            .into_string()
            .expect(error_str),
        context
    ).expect(format!("Could not parse template for yaml key: {}", key).as_str())
}

fn convert_body_hash(body: HashMap<String, String>) -> HashMap<String, Value> {
    let mut result: HashMap<String, Value> = HashMap::new();
    for (key, value) in body {
        if value == "true" || value == "false" {
            let bool_value = FromStr::from_str(&value)
                .expect("Could not convert boolean value in body");
            result.insert(key, Value::Bool(bool_value));
        } else if value == "[[empty]]" {
            // Do not insert empty values in body
            continue;
        } else {
            match serde_json::from_str::<Number>(&value) {
                Ok(n) => result.insert(key, Value::Number(n)),
                Err(_e) => result.insert(key, Value::String(value)),
            };
        }
    }
    result
}

fn print_response_json(result: &Value, pretty: bool) {
    if pretty {
        print!("{}", serde_json::to_string_pretty(result)
            .expect("Could not convert response to pretty print json"));
    } else {
        print!("{}", result);
    }
}

fn execute_request(app_name: &String, cmd_name: &str, yaml: &Yaml, subcmd_yaml: &Yaml, context: HashMap<String, HashMap<String, String>>) {
    let subcmd_hash = subcmd_yaml.clone().into_hash().expect("Could not hash subcmd yaml");
    let mut http_method: String;
    if subcmd_hash.contains_key(&Yaml::from_str("method")) {
        http_method = subcmd_yaml["method"].clone().into_string().unwrap();
    } else {
        http_method = String::from("get")
    }

    let mut headers = yaml::get_hash_from_yaml(&yaml["headers"], &context);

    let raw_body = yaml::get_hash_from_yaml(&subcmd_yaml["body"], &context);
    let body = convert_body_hash(raw_body);

    let query_params = yaml::get_hash_from_yaml(&subcmd_yaml["query_params"], &context);

    let endpoint = http::get_endpoint(&cmd_name, &context, &yaml, &query_params);

    let oauth_yaml = &yaml["oauth"];
    if !oauth_yaml.is_badvalue() {
        let client_id = get_parsed_yaml_key("client_id", &oauth_yaml, "Missing client_id", &context);
        let client_secret = get_parsed_yaml_key("client_secret", &oauth_yaml, "Missing client_secret", &context);
        let auth_url = get_parsed_yaml_key("auth_url", &oauth_yaml, "Missing auth_url", &context);
        let token_url = get_parsed_yaml_key("token_url", &oauth_yaml, "Missing token_url", &context);
        let oauth_token = oauth::get_oauth_token(
            app_name,
            client_id,
            client_secret,
            auth_url,
            token_url,
        );

        let header_name = get_parsed_yaml_key("header_key", &oauth_yaml, "Missing header_key", &context);
        headers.insert(header_name, oauth_token);
    }

    let mut response = http::request(&http_method, &endpoint, &headers, &body);
    let result: Value = response.json().expect(&format!("Could not convert response {:?} to json", response));

    // Raw output
    if context["args"].contains_key("raw_response") {
        print_response_json(&result, false);
        return
    }

    let mut template: String;
    if context["args"].contains_key("template") {
        template = context["args"]["template"].clone();
        if template == "json" {
            print_response_json(&result, true);
            return
        }
    } else if subcmd_hash.contains_key(&Yaml::from_str("response_template")) {
        template = subcmd_yaml["response_template"].clone().into_string().unwrap();
    } else {
        print_response_json(&result, true);
        return
    }

    let mut template_parser = template::Template::new(app_name); // TODO remove mut
    let mut response_context = HashMap::new();
    response_context.insert(String::from("response"), result.clone());
    if !context["args"].contains_key("quiet") {
        print!("{}", template_parser.get_compiled_template_with_context(template, response_context));
    }
}

fn execute_init(context: HashMap<String, HashMap<String, String>>) {
    let init_template = String::from(include_str!("../templates/config_template.yml"));
    let yaml_str = template::get_compiled_template_str_with_context(
        &init_template,
        &context)
        .expect("Could not create yaml template");
    let cmd_name = &context["args"]["PROJECT_NAME"];
    let filename = format!("{}.yml", cmd_name);
    fs::write(filename, yaml_str).expect("Unable to write file");
    println!("Config file {}.yml created", cmd_name);
    println!("To start testing with your extension create a symlink in your PATH targeting joat binaries with name: {}", cmd_name);
}

fn execute_auto_complete(mut app: App, app_name: &str, context: HashMap<String, HashMap<String, String>>) {
    let selected_shell = &context["args"]["SHELL"];
    let shell;
    match selected_shell.as_ref() {
        "zsh" => shell = Shell::Zsh,
        "bash" => shell = Shell::Bash,
        "fish" => shell = Shell::Fish,
        "powershell" => shell = Shell::PowerShell,
        "elvish" => shell = Shell::Elvish,
        _ => panic!("Unknown shell, use only lowercase"),
    };
    app.gen_completions(app_name,
                        shell,
                        ".")
}

fn execute(app: App, app_name: &String, cmd_name: &str, args: &ArgMatches, yaml: &Yaml) {
    let subcmd_yaml = yaml::get_subcommand_from_yaml(cmd_name, yaml);
    let script = &subcmd_yaml["script"];

    let vars_context = get_vars_context(yaml);
    let args_context = get_args_context(&args, &subcmd_yaml);
    let mut context = HashMap::new();
    context.insert(String::from("vars"), vars_context);
    context.insert(String::from("args"), args_context);

    if app_name == "joat" && cmd_name == "init" {
        execute_init(context);
        return;
    }

    if cmd_name == "auto_complete" {
        execute_auto_complete(app, app_name, context);
        return;
    }


    if !script.is_badvalue() {
        execute_script(context, &subcmd_yaml, &yaml);
    } else {
        execute_request(&app_name, &cmd_name, &yaml, &subcmd_yaml, context);
    }
}

fn install(cmd_name: &str, args: &ArgMatches, yaml: &Yaml) {
    println!("Install something");
}

fn format_cmd_name(cmd_name: &String) -> String {
    let re = Regex::new("[^/]*$").unwrap();
    String::from(
        re.find(cmd_name)
        .expect("Failed to parse main cmd name")
        .as_str()
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let app_name = format_cmd_name(&args[0]);
    let config_yaml = yaml::get_yaml_config(&app_name);

    let version;
    if app_name == env!("CARGO_PKG_NAME") {
        version = String::from(
            config_yaml["version"]
            .as_str()
            .expect("Version not defined"));
    } else {
        let djoat_version = env!("CARGO_PKG_VERSION");;
        let app_version = config_yaml["version"].as_str().expect("Version not defined");
        version = format!("{} (joat {})", app_version, djoat_version);
    }
    let mut app = App::from_yaml(&config_yaml)
        .version(&*version);

    let matches = app.clone().get_matches();

    match matches.subcommand() {
        (name, sub_cmd_option) => {
            match sub_cmd_option {
                Some(sub_cmd) => {
                    if name == "install" {
                        install(name, sub_cmd, &config_yaml)
                    } else {
                        execute(app, &app_name, name, sub_cmd, &config_yaml)
                    }
                },
                _ => {
                    // Could not find command, just print help
                    app.print_help().unwrap();
                    ::std::process::exit(1)
                }
            }
        }
    }
}