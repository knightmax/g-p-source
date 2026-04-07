#[cfg(test)]
mod tests {
    use g_p_source::parser::source_parser::SourceParser;
    use g_p_source::parser::language_registry::SupportedLanguage;
    use g_p_source::parser::symbol::SymbolKind;

    fn extract(source: &str, lang: SupportedLanguage) -> Vec<(String, SymbolKind)> {
        let mut parser = SourceParser::new().unwrap();
        let tree = parser.parse(source.as_bytes(), lang, None).unwrap();
        let symbols = parser.extract_symbols(&tree, source.as_bytes(), lang, "test.file");
        symbols.into_iter().map(|s| (s.name, s.kind)).collect()
    }

    #[test]
    fn java_class_and_method() {
        let source = r#"
public class UserService {
    public void findById(int id) {}
    private String name;
}
"#;
        let symbols = extract(source, SupportedLanguage::Java);
        assert!(symbols.iter().any(|(n, k)| n == "UserService" && *k == SymbolKind::Class));
        assert!(symbols.iter().any(|(n, k)| n == "findById" && *k == SymbolKind::Method));
    }

    #[test]
    fn java_interface_and_enum() {
        let source = r#"
public interface Serializable {
    void serialize();
}
public enum Color { RED, GREEN, BLUE }
"#;
        let symbols = extract(source, SupportedLanguage::Java);
        assert!(symbols.iter().any(|(n, k)| n == "Serializable" && *k == SymbolKind::Interface));
        assert!(symbols.iter().any(|(n, k)| n == "Color" && *k == SymbolKind::Enum));
    }

    #[test]
    fn typescript_class_and_function() {
        let source = r#"
export class UserDTO {
    name: string;
}
export function createUser(name: string): UserDTO {
    return new UserDTO();
}
export interface Config {
    port: number;
}
"#;
        let symbols = extract(source, SupportedLanguage::TypeScript);
        assert!(symbols.iter().any(|(n, k)| n == "UserDTO" && *k == SymbolKind::Class));
        assert!(symbols.iter().any(|(n, k)| n == "createUser" && *k == SymbolKind::Function));
        assert!(symbols.iter().any(|(n, k)| n == "Config" && *k == SymbolKind::Interface));
    }

    #[test]
    fn python_class_and_function() {
        let source = r#"
class UserService:
    def find_by_id(self, id):
        pass

def main():
    pass
"#;
        let symbols = extract(source, SupportedLanguage::Python);
        assert!(symbols.iter().any(|(n, k)| n == "UserService" && *k == SymbolKind::Class));
        assert!(symbols.iter().any(|(n, k)| n == "main" && *k == SymbolKind::Function));
    }

    #[test]
    fn rust_struct_enum_trait() {
        let source = r#"
pub struct Config {
    port: u16,
}
pub enum Status { Active, Inactive }
pub trait Serializable {
    fn serialize(&self) -> Vec<u8>;
}
pub fn main() {}
mod utils {}
"#;
        let symbols = extract(source, SupportedLanguage::Rust);
        assert!(symbols.iter().any(|(n, k)| n == "Config" && *k == SymbolKind::Struct));
        assert!(symbols.iter().any(|(n, k)| n == "Status" && *k == SymbolKind::Enum));
        assert!(symbols.iter().any(|(n, k)| n == "Serializable" && *k == SymbolKind::Trait));
        assert!(symbols.iter().any(|(n, k)| n == "main" && *k == SymbolKind::Function));
        assert!(symbols.iter().any(|(n, k)| n == "utils" && *k == SymbolKind::Module));
    }

    #[test]
    fn csharp_class_and_interface() {
        let source = r#"
namespace MyApp {
    public class UserService {
        public void FindById(int id) {}
    }
    public interface IRepository {
        void Save();
    }
}
"#;
        let symbols = extract(source, SupportedLanguage::CSharp);
        assert!(symbols.iter().any(|(n, k)| n == "UserService" && *k == SymbolKind::Class));
        assert!(symbols.iter().any(|(n, k)| n == "IRepository" && *k == SymbolKind::Interface));
    }
}
