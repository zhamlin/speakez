#[derive(PartialEq, Eq)]
pub enum EnumType {
    Zero,
    Struct,
    Tuple(String),
}

impl EnumType {
    pub fn name(&self) -> Option<&str> {
        match self {
            EnumType::Zero => None,
            EnumType::Struct => None,
            EnumType::Tuple(s) => Some(s),
        }
    }
}

pub struct EnumValue {
    pub name: String,
    pub value: u16,
    pub typ: EnumType,
}

fn parse_enum_variants(text: &str, needle: &str) -> Vec<EnumValue> {
    let (_, variants, _) = split_twice(text, needle, "}").unwrap();
    let mut value = 0;
    let variants: Vec<EnumValue> = variants
        .lines()
        .filter_map(|line| {
            let line = line.trim().trim_end_matches(',');
            let has_parens = line.contains("(") && line.contains(")");
            let mut typ = EnumType::Zero;

            let name = if has_parens {
                let p = line.find("(").expect("line should have parens");
                let (variant_name, _) = line.split_at(p);
                let (_, value_name, _) =
                    split_twice(line, "(", ")").expect("tuple enum variant should have parens");
                typ = EnumType::Tuple(value_name.to_string());
                variant_name
            } else if typ == EnumType::Zero {
                line.split("=")
                    .collect::<Vec<_>>()
                    .first()
                    .copied()
                    .unwrap_or(line)
            } else {
                return None;
            };

            if name.is_empty() {
                return None;
            }

            let val = EnumValue {
                name: name.trim().to_string(),
                value,
                typ,
            };
            value += 1;
            Some(val)
        })
        .collect();
    variants
}

type FormatterFn = fn(&[EnumValue], &str, &str) -> String;

pub struct RegionGenerator {
    region_name: String,
    formatter: FormatterFn,
}

impl RegionGenerator {
    pub fn new(region_name: impl Into<String>, formatter: FormatterFn) -> Self {
        Self {
            region_name: region_name.into(),
            formatter,
        }
    }
}

fn apply_region_generators(
    mut text: String,
    variants: &[EnumValue],
    output_indent: &str,
    enum_name: &str,
    generators: &[RegionGenerator],
) -> String {
    for generator in generators {
        let arms = (generator.formatter)(variants, output_indent, enum_name);
        let start_marker = format!("{output_indent}// region:{}\n", generator.region_name);
        let end_marker = format!("\n{output_indent}// endregion:{}\n", generator.region_name);

        if let Some((prefix, _, suffix)) = split_twice(&text, &start_marker, &end_marker) {
            text = format!(
                "{prefix}{start_marker}{}{end_marker}{suffix}",
                arms.trim_end()
            );
        } else {
            panic!("region not found: {}:", &generator.region_name);
        }
    }
    text
}

// https://matklad.github.io/2022/03/26/self-modifying-code.html
pub fn sourcegen_from_code(
    path: &std::path::Path,
    enum_name: &str,
    indent: usize,
    generators: &[RegionGenerator],
) {
    let original_text = std::fs::read_to_string(path).unwrap();

    let enum_stmt = format!("enum {enum_name} {{\n");

    let variants = parse_enum_variants(&original_text, &enum_stmt);

    let (prefix, _, _) = split_twice(&original_text, &enum_stmt, "}").unwrap();
    let enum_indent = prefix
        .lines()
        .last()
        .map(|line| line.len() - line.trim_start().len())
        .unwrap_or(0);
    let indent = " ".repeat(enum_indent + indent);

    let new_text =
        apply_region_generators(original_text, &variants, &indent, enum_name, generators);

    crate::file_matches(path, &new_text);
}

fn split_twice<'a>(
    text: &'a str,
    start_marker: &str,
    end_marker: &str,
) -> Option<(&'a str, &'a str, &'a str)> {
    let (prefix, rest) = text.split_once(start_marker)?;
    let (mid, suffix) = rest.split_once(end_marker)?;
    Some((prefix, mid, suffix))
}
