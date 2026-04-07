// Tree-sitter S-expression queries for each supported language.
// Each query captures symbol definitions for extraction.

pub const JAVA_QUERY: &str = r#"
(class_declaration
  name: (identifier) @name) @definition.class

(interface_declaration
  name: (identifier) @name) @definition.interface

(enum_declaration
  name: (identifier) @name) @definition.enum

(method_declaration
  name: (identifier) @name) @definition.method

(constructor_declaration
  name: (identifier) @name) @definition.method

(field_declaration
  declarator: (variable_declarator
    name: (identifier) @name)) @definition.constant

(import_declaration
  (scoped_identifier) @name) @definition.import
"#;

pub const TYPESCRIPT_QUERY: &str = r#"
(class_declaration
  name: (type_identifier) @name) @definition.class

(interface_declaration
  name: (type_identifier) @name) @definition.interface

(function_declaration
  name: (identifier) @name) @definition.function

(method_definition
  name: (property_identifier) @name) @definition.method

(type_alias_declaration
  name: (type_identifier) @name) @definition.type_alias

(enum_declaration
  name: (identifier) @name) @definition.enum

(import_statement
  source: (string) @name) @definition.import
"#;

pub const PYTHON_QUERY: &str = r#"
(class_definition
  name: (identifier) @name) @definition.class

(function_definition
  name: (identifier) @name) @definition.function

(import_statement
  name: (dotted_name) @name) @definition.import

(import_from_statement
  module_name: (dotted_name) @name) @definition.import
"#;

pub const RUST_QUERY: &str = r#"
(struct_item
  name: (type_identifier) @name) @definition.struct

(enum_item
  name: (type_identifier) @name) @definition.enum

(trait_item
  name: (type_identifier) @name) @definition.trait

(impl_item
  trait: (type_identifier)? @trait_name
  type: (type_identifier) @name) @definition.class

(function_item
  name: (identifier) @name) @definition.function

(mod_item
  name: (identifier) @name) @definition.module

(use_declaration
  argument: (_) @name) @definition.import
"#;

pub const CSHARP_QUERY: &str = r#"
(class_declaration
  name: (identifier) @name) @definition.class

(interface_declaration
  name: (identifier) @name) @definition.interface

(struct_declaration
  name: (identifier) @name) @definition.struct

(enum_declaration
  name: (identifier) @name) @definition.enum

(method_declaration
  name: (identifier) @name) @definition.method

(namespace_declaration
  name: (_) @name) @definition.namespace

(using_directive
  (_) @name) @definition.import
"#;
