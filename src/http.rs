use log::{debug, info};
use reqwest::Method;
use reqwest::Response;
use serde_json::value::Value;
use std::collections::HashMap;
use std::vec::Vec;

use crate::{template, Context};

fn get_complete_endpoint(base_endpoint: &str, path: &str) -> String {
    let mut endpoint = String::from(base_endpoint);
    endpoint.push_str(&path);
    return endpoint;
}

fn stringify(query: Vec<(String, String)>) -> String {
    let mut query_params = query.iter().fold(String::new(), |acc, tuple| {
        acc + &tuple.0 + "=" + &tuple.1 + "&"
    });
    query_params.pop();
    query_params
}

fn get_string_from_value(value: &Value) -> &str {
    match value.as_str() {
        Some(s) => s,
        None => "",
    }
}

fn get_endpoint_with_qp(
    endpoint: String,
    query_params: &HashMap<String, Value>,
    context: &Context,
) -> String {
    if endpoint.contains("?") || query_params.is_empty() {
        return endpoint;
    }
    let mut param_vec = Vec::new();
    for (key, value) in query_params {
        if value.is_null() {
            continue;
        }
        let parsed_qp_value = template::get_compiled_template_str_with_context(
            &get_string_from_value(value).to_string(),
            &context,
        )
        .expect("Error parsing query param option, check your template");
        param_vec.push((key.clone(), parsed_qp_value));
    }
    format!("{}?{}", endpoint, stringify(param_vec))
}

pub fn get_endpoint(
    endpoint: &str,
    path: &str,
    context: &Context,
    query_params: &HashMap<String, Value>,
) -> String {
    let raw_endpoint = get_complete_endpoint(endpoint, path);
    let endpoint_with_qp = get_endpoint_with_qp(raw_endpoint, query_params, context);
    let parsed_endpoint =
        template::get_compiled_template_str_with_context(&endpoint_with_qp, &context)
            .expect(format!("Could not parse endpoint {}", endpoint_with_qp).as_str());

    return parsed_endpoint;
}

fn get_method(method: &String) -> Method {
    if method == "GET" {
        return Method::GET;
    }
    if method == "PUT" {
        return Method::PUT;
    }
    if method == "POST" {
        return Method::POST;
    }
    if method == "PATCH" {
        return Method::PATCH;
    }
    if method == "DELETE" {
        return Method::DELETE;
    }
    return Method::GET;
}

pub fn request(
    method: &String,
    endpoint: &String,
    headers: &HashMap<String, Value>,
    body: &HashMap<String, Value>,
    form: &HashMap<String, Value>,
) -> Response {
    let client = reqwest::Client::new();
    let reqwest_method = get_method(&method);
    let mut request = client.request(reqwest_method, endpoint);
    for (name, value) in headers.iter() {
        let header_value = get_string_from_value(value);
        request = request.header(&name[..], header_value);
    }
    request = request.json(&body);
    if form.len() > 0 {
        request = request.form(&form);
    }
    info!("{:?}", request);
    let response = match request.send() {
        Ok(t) => t,
        Err(e) => {
            println!("Could not get response for endpoint {}", endpoint);
            println!("Error: {}", e);
            panic!("Failed request");
        }
    };
    debug!("Response {:?}", response);
    return response;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_string_from_yaml() {
        // Arrange
        let base_endpoint = "http://example.com".to_string();
        let path = "/path".to_string();

        // Act
        let endpoint = get_complete_endpoint(&base_endpoint, &path);

        // Assert
        assert_eq!("http://example.com/path", endpoint);
    }

    #[test]
    fn test_stringify() {
        // Arrange
        let mut query_params = Vec::new();
        query_params.push((String::from("foo"), String::from("bar")));
        query_params.push((String::from("foo"), String::from("bar")));

        // Act
        let query_params_str = stringify(query_params);

        // Assert
        assert_eq!("foo=bar&foo=bar", query_params_str);
    }

    #[test]
    fn test_get_endpoint_with_qp() {
        // Arrange
        let endpoint = String::from("http://example.com/path");
        let mut query_params = HashMap::new();
        query_params.insert(String::from("foo"), Value::String("bar".to_string()));
        let context = HashMap::new();

        // Act
        let endpoint = get_endpoint_with_qp(endpoint, &query_params, &context);

        // Assert
        assert_eq!("http://example.com/path?foo=bar", endpoint);
    }

    #[test]
    fn test_get_endpoint_with_qp_empty_qp() {
        // Arrange
        let endpoint = String::from("http://example.com/path");
        let query_params = HashMap::new();
        let context = HashMap::new();

        // Act
        let endpoint = get_endpoint_with_qp(endpoint, &query_params, &context);

        // Assert
        assert_eq!("http://example.com/path", endpoint);
    }

    #[test]
    fn test_get_endpoint() {
        // Arrange
        let base_endpoint = "http://example.com/";
        let path = "path/to/resource";
        let context = HashMap::new();
        let query_params = HashMap::new();

        // Act
        let endpoint = get_endpoint(&base_endpoint, &path, &context, &query_params);

        // Assert
        assert_eq!(endpoint, format!("{}{}", base_endpoint, path));
    }
}
