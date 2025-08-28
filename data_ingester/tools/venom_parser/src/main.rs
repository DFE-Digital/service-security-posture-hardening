#![feature(str_split_remainder)]
static data: &str = r#"
name: HTTP security response headers test suites
vars:
  target_site: ''
  logout_url: ''
  request_timeout_in_seconds: 20
testcases:
  - name: Strict-Transport-Security
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.Strict-Transport-Security ShouldNotBeNil
          - or:
              - result.headers.Strict-Transport-Security ShouldEqual "max-age=31536000; includeSubDomains"
              - result.headers.Strict-Transport-Security ShouldEqual "max-age=31536000; includeSubDomains; preload"
  - name: X-Frame-Options
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.X-Frame-Options ShouldNotBeNil
          - result.headers.X-Frame-Options ShouldBeIn "deny" "DENY"
  - name: X-Content-Type-Options
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.X-Content-Type-Options ShouldNotBeNil
          - result.headers.X-Content-Type-Options ShouldEqual "nosniff"
  - name: Content-Security-Policy
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.Content-Security-Policy ShouldNotBeNil
          - result.headers.Content-Security-Policy ShouldNotContainSubstring "unsafe"
  - name: X-Permitted-Cross-Domain-Policies
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.X-Permitted-Cross-Domain-Policies ShouldNotBeNil
          - result.headers.X-Permitted-Cross-Domain-Policies ShouldEqual "none"
  - name: Referrer-Policy
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.Referrer-Policy ShouldNotBeNil
          - result.headers.Referrer-Policy ShouldEqual "no-referrer"
  - name: Clear-Site-Data
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}/{{.logout_url}}'
        skip_body: true
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.Clear-Site-Data ShouldNotBeNil
          - result.headers.Clear-Site-Data ShouldEqual '"cache","cookies","storage"'
  - name: Cross-Origin-Embedder-Policy
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.Cross-Origin-Embedder-Policy ShouldNotBeNil
          - result.headers.Cross-Origin-Embedder-Policy ShouldEqual "require-corp"
  - name: Cross-Origin-Opener-Policy
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.Cross-Origin-Opener-Policy ShouldNotBeNil
          - result.headers.Cross-Origin-Opener-Policy ShouldEqual "same-origin"
  - name: Cross-Origin-Resource-Policy
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.Cross-Origin-Resource-Policy ShouldNotBeNil
          - result.headers.Cross-Origin-Resource-Policy ShouldEqual "same-origin"
  - name: Permissions-Policy
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.Permissions-Policy ShouldNotBeNil
          - result.headers.Permissions-Policy ShouldContainSubstring accelerometer=()
          - result.headers.Permissions-Policy ShouldContainSubstring autoplay=()
          - result.headers.Permissions-Policy ShouldContainSubstring camera=()
          - result.headers.Permissions-Policy ShouldContainSubstring clipboard-read=()
          - result.headers.Permissions-Policy ShouldContainSubstring clipboard-write=()
          - result.headers.Permissions-Policy ShouldContainSubstring cross-origin-isolated=()
          - result.headers.Permissions-Policy ShouldContainSubstring display-capture=()
          - result.headers.Permissions-Policy ShouldContainSubstring encrypted-media=()
          - result.headers.Permissions-Policy ShouldContainSubstring fullscreen=()
          - result.headers.Permissions-Policy ShouldContainSubstring gamepad=()
          - result.headers.Permissions-Policy ShouldContainSubstring geolocation=()
          - result.headers.Permissions-Policy ShouldContainSubstring gyroscope=()
          - result.headers.Permissions-Policy ShouldContainSubstring hid=()
          - result.headers.Permissions-Policy ShouldContainSubstring idle-detection=()
          - result.headers.Permissions-Policy ShouldContainSubstring interest-cohort=()
          - result.headers.Permissions-Policy ShouldContainSubstring keyboard-map=()
          - result.headers.Permissions-Policy ShouldContainSubstring magnetometer=()
          - result.headers.Permissions-Policy ShouldContainSubstring microphone=()
          - result.headers.Permissions-Policy ShouldContainSubstring midi=()
          - result.headers.Permissions-Policy ShouldContainSubstring payment=()
          - result.headers.Permissions-Policy ShouldContainSubstring picture-in-picture=()
          - result.headers.Permissions-Policy ShouldContainSubstring publickey-credentials-get=()
          - result.headers.Permissions-Policy ShouldContainSubstring screen-wake-lock=()
          - result.headers.Permissions-Policy ShouldContainSubstring serial=()
          - or:
              - result.headers.Permissions-Policy ShouldContainSubstring sync-xhr=(self)
              - result.headers.Permissions-Policy ShouldContainSubstring sync-xhr=()
          - result.headers.Permissions-Policy ShouldContainSubstring unload=()
          - result.headers.Permissions-Policy ShouldContainSubstring usb=()
          - result.headers.Permissions-Policy ShouldContainSubstring web-share=()
          - result.headers.Permissions-Policy ShouldContainSubstring xr-spatial-tracking=()

  - name: Cache-Control
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.Cache-Control ShouldNotBeNil
          - 'result.headers.Cache-Control ShouldEqual "no-store, max-age=0"'
  - name: X-DNS-Prefetch-Control
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.X-Dns-Prefetch-Control ShouldNotBeNil
          - result.headers.X-Dns-Prefetch-Control ShouldEqual "off"
  - name: Feature-Policy (should not exist)
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        info: >-
          This header has now been renamed to Permissions-Policy in the
          specification.
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.Feature-Policy ShouldBeNil
  - name: Public-Key-Pins (should not exist)
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        info: >-
          This header has been deprecated by all major browsers and is no longer
          recommended. Avoid using it, and update existing code if possible!
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.Public-Key-Pins ShouldBeNil
  - name: Expect-CT (should not exist)
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        info: >-
          This header will likely become obsolete in June 2021. Since May 2018
          new certificates are expected to support SCTs by default. Certificates
          before March 2018 were allowed to have a lifetime of 39 months, those
          will all be expired in June 2021.
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.Expect-CT ShouldBeNil
  - name: X-XSS-Protection (should not exist)
    steps:
      - type: http
        method: GET
        url: '{{.target_site}}'
        skip_body: true
        info: >-
          The X-XSS-Protection header has been deprecated by modern browsers and
          its use can introduce additional security issues on the client side.
        timeout: '{{.request_timeout_in_seconds}}'
        assertions:
          - result.statuscode ShouldEqual 200
          - result.headers.X-XSS-Protection ShouldBeNil
"#;
use serde_json::Value;
//use serde_yaml::Value;
use serde::Deserialize;

fn main() {
    //let pdata: Value = serde_yaml::from_str(&data).unwrap();
    //dbg!(pdata);
    let pdata: Venom = serde_yaml::from_str(&data).unwrap();
    dbg!(&pdata);

    for testcase in pdata.testcases {
	// dbg!(testcase.name);
	for step in &testcase.steps {
	    for assertion in &step.assertions {
		match assertion {
		    Assertion::S(assert) => {
			println!("{}", assert.as_splunk(&testcase.name().to_owned()));
		    },
		    Assertion::Or(or) => {
			println!("{}", or.as_splunk(&testcase.name().to_owned()));
		    },
		}

	    }
	}
    }
}

#[derive(Deserialize, Debug)]
struct Venom {
    testcases: Vec<TestCase>
}

#[derive(Deserialize, Debug)]
struct TestCase {
    name: String,
    steps: Vec<Step>
}

impl TestCase {
    fn name(&self) -> &str {
	self.name.split(" ").next().unwrap()
    }
}

#[derive(Deserialize, Debug)]
struct Step {
    //type: String,
    assertions: Vec<Assertion>}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Assertion {
    S(Assert),
    Or(Or)
}

#[derive(Deserialize, Debug)]
struct Assert(String);

impl AssertTrait for Assert{
    fn inner(&self) -> &str {
	&self.0
    }
} 

trait AssertTrait {
    fn inner(&self) -> &str;
    
    fn parts(&self) -> Vec<&str> {
	self.inner().split(" ").collect()
    }


    fn key_name(&self) -> String {
	let part = self.parts()[0].split(".").last().unwrap();
	if part == "statuscode" {
	    "status".into()
	} else {
	    format!("headers.{}", part.to_ascii_lowercase())
	}
    }

    fn operation(&self) -> &str {
	self.parts()[1]	
    }

    fn value(&self)  -> String {
	let mut iter = self.inner().split(" ");
	iter.next();
	iter.next();
	let mut v = iter.remainder().unwrap().to_owned();
	v = v.trim_end_matches('"').to_string();
	v = v.trim_start_matches('"').to_string();	

	v = v.replace("'", "");
	v = v.replace('"', r#"\""#);
	// dbg!(&v);
	v
    }

    fn value_without_trim(&self) -> String {
	let mut iter = self.inner().split(" ");
	iter.next();
	iter.next();
	iter.remainder().unwrap().to_owned()
    }

    fn field_prefix(&self)  -> &str {
	"headercheck"
    }

    fn as_test(&self) -> String {
	let op: String = match self.operation() {
	    "ShouldBeIn" => self.should_be_in(),
	    "ShouldBeNil" => self.should_be_nil(),
	    "ShouldContainSubstring" => self.should_contain_substring().to_string(),
	    "ShouldEqual" => self.should_equal().to_string(),
	    "ShouldNotBeNil" => self.should_not_be_nil(),
	    "ShouldNotContainSubstring" => self.should_not_contain_substring(),
	    _ => panic!("unknown operation: {}", self.operation()),
	};
	op
    }

    fn as_splunk(&self, name: &str) -> String {
	let op = self.as_test();
	let field_prefix = self.field_prefix();
	format!("| eval {field_prefix}.{name}=mvappend('{field_prefix}.{name}', {op})")
    }

    fn should_be_in(&self) -> String {
	let key_name = self.key_name();
	let value= self.value_without_trim().split(' ').collect::<Vec<_>>().join(", ");
	let result_key_value = value.replace('"', "").replace(' ', "_OR_").replace(',',"");
	let result_key = format!("{key_name}:should_be:{result_key_value}");
	format!(
	    "if(in('{key_name}', {value}), \"{result_key}=pass\", \"{result_key}=fail\")")	
    }

    fn should_not_contain_substring(&self) -> String {
	let key_name = self.key_name();
	let value= self.value();
	let result_key = format!("{key_name}:should_not_contain:{value}");
	format!(
	    "if(NOT like('{key_name}', \"%{value}%\"), \"{result_key}=pass\", \"{result_key}=fail\")"
	)
    }    
    

    fn should_contain_substring(&self) -> SplunkTestCase {
	let key_name = self.key_name();
	let value=self.value().replace('(', "").replace(')', "");
	let result_key = format!("{key_name}:should_contain:{value}");

	let test_case = SplunkTestCase {
	    test: format!("like('{key_name}', \"%{value}%\")"),
	    result_key: result_key
	};
	test_case
    }     

    fn should_not_be_nil(&self) -> String {
	let key_name = self.key_name();
	let result_key = format!("{key_name}:should_not_be_empty");		
	format!(
	    "if(isnotnull('{key_name}'), \"{result_key}=pass\", \"{result_key}=fail\")"
	)
    }    

    fn should_be_nil(&self) -> String {
	let key_name = self.key_name();
	let result_key = format!("{key_name}:should_be_empty");			
	format!(
	    "if(isnull('{key_name}'), \"{result_key}=pass\", \"{result_key}=fail\")"
	)
    }    

    fn should_equal(&self) -> SplunkTestCase {
	let key_name = self.key_name();
	let value=self.value();	
	let result_key = format!("{key_name}:should_be:{value}");
	let test_case = SplunkTestCase {
	    test: format!("'{key_name}' = \"{value}\""),
	    result_key: result_key
	};
	test_case
	// format!(
	//     "if('{key_name}' = \"{value}\", \"{result_key}=pass\", \"{result_key}=fail\")",
//	)
    }
}

struct SplunkTestCase {
    // key_name: String,
    // value: String,
    result_key: String,
    test: String,
    // pass: String,
    // fail: String,
}

impl SplunkTestCase {
    fn to_string(&self) -> String {
	format!(
	    "if({0}, \"{1}=pass\", \"{1}=fail\")",
	self.test, self.result_key)
    }


}

#[derive(Deserialize, Debug)]
struct Or {
    or: Vec<Assert>
}

impl Or {

    fn as_splunk(&self, name: &str) -> String {
	let mut tests: String = String::new();
	let mut result_key: String = String::new();
	let test_cases: Vec<SplunkTestCase> = self.or.iter().map(|assert| {
	    match assert.operation() {
		//"ShouldBeIn" => self.should_be_in(),
		//"ShouldBeNil" => self.should_be_nil(),
		"ShouldContainSubstring" => assert.should_contain_substring(),
		"ShouldEqual" => assert.should_equal(),
		//"ShouldNotBeNil" => self.should_not_be_nil(),
		//"ShouldNotContainSubstring" => self.should_not_contain_substring(),
		_ => panic!("unknown operation: {}", assert.operation()),
	    }
	}).collect();

	let tests = test_cases.iter().map(|tc| tc.test.as_str()).collect::<Vec<_>>().join(" OR ");
	let result_key = test_cases.iter().map(|tc| tc.result_key.as_str()).collect::<Vec<_>>().join("_OR_");	

	let op = format!(
	    "if({0}, \"{1}=pass\", \"{1}=fail\")",
	    tests, result_key);

	let field_prefix = self.or.first().unwrap().field_prefix();
	format!("| eval {field_prefix}.{name}=mvappend('{field_prefix}.{name}', {op})")
    }
}
