use proc_macro::TokenStream;
use std::fmt::*;
use syn::{parse_macro_input, LitStr};
macro_rules! CSR_ACCESSOR {
    () => {
        r#"
pub mod csr{{
    pub const CSR_ID: usize = {};
    #[inline]
    pub unsafe fn csrrw(rs1: usize)->usize{{
        let mut rd;
        llvm_asm!("csrrw $0, $2, $1" :"=r"(rd): "r"(rs1), "i"(CSR_ID) :: "volatile");
        rd
    }}
    #[inline]
    pub unsafe fn csrrw_x0(rs1: usize){{
        llvm_asm!("csrrw x0, $1, $0" :: "r"(rs1), "i"(CSR_ID) :: "volatile");
    }}
    #[inline]
    pub unsafe fn csrrs(rs1: usize)->usize{{
        let mut rd;
        llvm_asm!("csrrs $0, $2, $1" :"=r"(rd): "r"(rs1), "i"(CSR_ID) :: "volatile");
        rd
    }}
    #[inline]
    pub unsafe fn csrrs_x0()->usize{{
        let mut rd;
        llvm_asm!("csrrs $0, $1, x0" :"=r"(rd): "i"(CSR_ID) :: "volatile");
        rd
    }}
    #[inline]
    pub unsafe fn csrrc(rs1: usize)->usize{{
        let mut rd;
        llvm_asm!("csrrc $0, $2, $1" :"=r"(rd): "r"(rs1), "i"(CSR_ID) :: "volatile");
        rd
    }}
    #[inline]
    pub unsafe fn csrrc_x0()->usize{{
        let mut rd;
        llvm_asm!("csrrc $0, $1, x0" :"=r"(rd): "i"(CSR_ID) :: "volatile");
        rd
    }}
}}
"#
    };
}

macro_rules! as_str_polyfill {
    ($x: expr, $r: expr) => {{
        let mut y = $x.clone();
        if let Some(x) = y.next() {
            $r.split_at(x.as_ptr() as usize - $r.as_ptr() as usize).1
        } else {
            ""
        }
    }};
}
#[derive(Debug, Clone)]
struct EnumerationDescriptor<'a> {
    enumerations: Vec<(&'a str, usize)>,
}
impl<'a> EnumerationDescriptor<'a> {
    pub fn parse(enums: &'a str) -> Self {
        let mut counter = 0;
        let list = enums.split(";");
        let mut e = Vec::new();
        for tup in list {
            let mut t = tup.split("=");
            let n = t.next().unwrap();
            if let Some(new_id) = t.next() {
                counter = new_id.parse().unwrap();
            }
            e.push((n, counter));
            counter += 1;
        }
        EnumerationDescriptor { enumerations: e }
    }
    fn generate_enum(&self, name: &str) -> String {
        let mut ret = String::new();
        write!(
            &mut ret,
            "#[derive(Copy, Clone, Debug)]
#[repr(usize)]
"
        )
        .unwrap();
        write!(&mut ret, "pub enum {}{{\n", name).unwrap();
        let mut branches = String::new();
        for e in self.enumerations.iter() {
            write!(&mut ret, "    {} = {},\n", e.0, e.1).unwrap();
            write!(&mut branches, "            {} => Self::{},\n", e.1, e.0).unwrap();
        }

        write!(
            &mut ret,
            "}}
impl {}{{
    #[inline]
    fn from_usize(x: usize)->Self{{
        match x{{
{}            _ => unreachable!()
        }}
    }}
}}
",
            name, branches
        )
        .unwrap();
        return ret;
    }
}
#[derive(Debug, Clone)]
struct BitFieldDescriptor<'a> {
    name: &'a str,
    description: &'a str,
    lo: usize,
    hi: usize,
    ed: Option<(&'a str, EnumerationDescriptor<'a>)>,
}

impl<'a> BitFieldDescriptor<'a> {
    pub fn parse(desc: &'a str) -> Self {
        let mut parts = desc.split(",");
        let name = parts.next().unwrap();
        let hi = parts.next().unwrap().parse::<usize>().unwrap();
        let lo = parts.next().unwrap().parse::<usize>().unwrap();
        let (lo, hi) = if lo < hi { (lo, hi) } else { (hi, lo) };
        let use_enum = parts.next().unwrap();
        let ed = if use_enum != "number" {
            let opts = parts.next().unwrap();
            Some((use_enum, EnumerationDescriptor::parse(opts)))
        } else {
            None
        };
        let description = as_str_polyfill!(parts, desc);
        BitFieldDescriptor {
            name,
            lo,
            hi,
            description,
            ed,
        }
    }
    pub fn generate_enum(&self) -> Option<String> {
        if let Some((n, e)) = &self.ed {
            Some(e.generate_enum(n))
        } else {
            None
        }
    }
    pub fn flag_type(&self) -> &str {
        if let Some((n, _)) = self.ed {
            n
        } else {
            if self.lo == self.hi {
                "bool"
            } else {
                "usize"
            }
        }
    }
    fn mask(&self) -> String {
        format!("{}", (1usize << (self.hi - self.lo + 1)) - 1)
    }
    fn generate_read_write(&self) -> String {
        format!(
            "
#[inline]
pub fn read_{}(&self)->{}{{
    {}::from_usize(((self.0>>{}) & {}))
}}
#[inline]
pub fn write_{}(&mut self, val: {}){{
    assert_eq!(val as usize & {}, val as usize, \"Too long input for write_{}!\");
    self.0 = (self.0 & !({} << {}))|((val as usize) << {})
}}\n",
            self.name,
            self.flag_type(),
            self.flag_type(),
            self.lo,
            self.mask(),
            self.name,
            self.flag_type(),
            self.mask(),
            self.name,
            self.mask(),
            self.lo,
            self.lo
        )
    }

    fn generate_bit_set(&self) -> String {
        format!(
            "
#[inline]
pub fn set_{}()->bool{{
    unsafe {{csr::csrrc({}) & {} !=0}}
}}
#[inline]
pub fn clear_{}()->bool{{
    unsafe {{csr::csrrs({}) & {} !=0 }}
}}\n",
            self.name,
            1usize << self.lo,
            1usize << self.lo,
            self.name,
            1usize << self.lo,
            1usize << self.lo
        )
    }
}

#[derive(Debug, Clone)]
struct CSRDescriptor<'a> {
    name: &'a str,
    id: usize,
    description: &'a str,
    bfs: Vec<BitFieldDescriptor<'a>>,
}

impl<'a> CSRDescriptor<'a> {
    pub fn parse(d: &'a str) -> Self {
        let mut parts = d.split("\n");
        let name = parts.next().unwrap();
        let id = parts.next().unwrap().parse::<usize>().unwrap();
        let mut bfs = Vec::new();
        while let Some(x) = parts.next() {
            if x == "end" {
                break;
            } else {
                bfs.push(BitFieldDescriptor::parse(x));
            }
        }
        CSRDescriptor {
            name,
            id,
            description: as_str_polyfill!(parts, d),
            bfs,
        }
    }
    pub fn generate(&self) -> String {
        let mut trait_impls = String::new();
        let mut enums = String::new();
        let mut debug_fields = String::new();
        for bf in self.bfs.iter() {
            if bf.lo == bf.hi {
                write!(&mut trait_impls, "{}", bf.generate_bit_set()).unwrap();
            }
            write!(&mut trait_impls, "{}", bf.generate_read_write()).unwrap();
            if let Some(x) = bf.generate_enum() {
                write!(&mut enums, "{}", x).unwrap();
            }
            write!(
                &mut debug_fields,
                "         .field(\"{}\", &self.read_{}())\n",
                bf.name, bf.name
            )
            .unwrap();
        }
        format!(
            "
use super::BoolExt;
/// {}
#[derive(Copy, Clone)]
pub struct {}(pub usize);
impl {}{{
#[inline]
pub fn read()->Self{{
    {}(unsafe{{csr::csrrs_x0()}})
}}
#[inline]
pub fn write(self){{
    unsafe{{csr::csrrw_x0(self.0)}};
}}
#[inline]
pub fn replace(self)->Self{{
    {}(unsafe{{csr::csrrw(self.0)}})
}}
{}
}}
// enums
{}
// csr mod
{}
// Debug
impl core::fmt::Debug for {} {{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{
        f.debug_struct(\"{}\")
{}
         .finish()
    }}
}}
",
            self.description,
            self.name,
            self.name,
            self.name,
            self.name,
            trait_impls,
            enums,
            format!(CSR_ACCESSOR!(), self.id),
            self.name,
            self.name,
            debug_fields
        )
    }
}

#[proc_macro]
pub fn generate_csr(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as LitStr);
    CSRDescriptor::parse(&input.value())
        .generate()
        .parse()
        .unwrap()
}

const HFENCE_GVMA_TEMPLATE: u32 = 0b0110001_00000_00000_000_00000_1110011;
const HFENCE_VVMA_TEMPLATE: u32 = 0b0010001_00000_00000_000_00000_1110011;
const HLV_B_TEMPLATE: u32 = 0b0110000_00000_00000_100_00000_1110011;
const HLV_BU_TEMPLATE: u32 = 0b0110000_00001_00000_100_00000_1110011;
const HLV_H_TEMPLATE: u32 = 0b0110010_00000_00000_100_00000_1110011;
const HLV_HU_TEMPLATE: u32 = 0b0110010_00001_00000_100_00000_1110011;
const HLVX_HU_TEMPLATE: u32 = 0b0110010_00011_00000_100_00000_1110011;
const HLV_W_TEMPLATE: u32 = 0b0110100_00000_00000_100_00000_1110011;
const HLVX_WU_TEMPLATE: u32 = 0b0110100_00011_00000_100_00000_1110011;
const HSV_B_TEMPLATE: u32 = 0b0110001_00000_00000_100_00000_1110011;
const HSV_H_TEMPLATE: u32 = 0b0110011_00000_00000_100_00000_1110011;
const HSV_W_TEMPLATE: u32 = 0b0110101_00000_00000_100_00000_1110011;
const HLV_WU_TEMPLATE: u32 = 0b0110100_00001_00000_100_00000_1110011;
const HLV_D_TEMPLATE: u32 = 0b0110110_00000_00000_100_00000_1110011;
const HSV_D_TEMPLATE: u32 = 0b0110111_00000_00000_100_00000_1110011;
#[derive(Copy, Clone, Debug)]
enum Register {
    x0 = 0,
    a0 = 10,
    a1 = 11,
}
fn template_rs1_rs2(template: u32, rs1: Register, rs2: Register) -> (u32, bool) {
    (
        ((rs2 as u32) << 20) | ((rs1 as u32) << 15) | template,
        false,
    )
}
fn template_rs1_rd(template: u32, rs1: Register, rd: Register) -> (u32, bool) {
    (((rs1 as u32) << 15) | ((rd as u32) << 7) | template, true)
}
fn emit_instruction<T: std::fmt::Write>(
    writer: &mut T,
    writer_rust: &mut T,
    name: &str,
    insn: (u32, bool),
) -> std::fmt::Result {
    write!(
        writer,
        ".global invoke_insn_{}\ninvoke_insn_{}:\n    .word {}\n    ret\n",
        name, name, insn.0
    )?;
    write!(
        writer_rust,
        "    pub fn invoke_insn_{}(rs1: usize, {}: usize);\n",
        name,
        if insn.1 { "rd" } else { "rs2" }
    )
}

// Generate instructions on the fly.
#[proc_macro]
pub fn generate_inline_asm(item: TokenStream) -> TokenStream {
    //let mut file = std::fs::File::create("asm.S").unwrap();
    let mut file = String::new();
    let mut file_rs = String::new();
    write!(&mut file_rs, "extern \"C\" {{\n").unwrap();
    macro_rules! gen {
        (rs2, $name: expr, $template: ident) => {
            emit_instruction(
                &mut file,
                &mut file_rs,
                $name,
                template_rs1_rs2($template, Register::a0, Register::a1),
            )
            .unwrap();
        };
        (rd, $name: expr, $template: ident) => {
            emit_instruction(
                &mut file,
                &mut file_rs,
                $name,
                template_rs1_rd($template, Register::a0, Register::a0),
            )
            .unwrap();
        };
    }
    gen!(rs2, "hfence_gvma", HFENCE_GVMA_TEMPLATE);
    gen!(rs2, "hfence_vvma", HFENCE_VVMA_TEMPLATE);
    gen!(rs2, "hlv_b", HLV_B_TEMPLATE);
    gen!(rs2, "hlv_bu", HLV_BU_TEMPLATE);
    gen!(rs2, "hlv_h", HLV_H_TEMPLATE);
    gen!(rs2, "hlv_hu", HLV_HU_TEMPLATE);
    gen!(rs2, "hlvx_hu", HLVX_HU_TEMPLATE);
    gen!(rs2, "hlv_w", HLV_W_TEMPLATE);
    gen!(rs2, "hlvx_wu", HLVX_WU_TEMPLATE);
    gen!(rd, "hsv_b", HSV_B_TEMPLATE);
    gen!(rd, "hsv_h", HSV_H_TEMPLATE);
    gen!(rd, "hsv_w", HSV_W_TEMPLATE);
    gen!(rs2, "hlv_wu", HLV_WU_TEMPLATE);
    gen!(rs2, "hlv_d", HLV_D_TEMPLATE);
    gen!(rd, "hsv_d", HSV_D_TEMPLATE);

    write!(
        &mut file_rs,
        "}}
global_asm!(r#\"
{}
\"#);
",
        file
    )
    .unwrap();
    file_rs.parse().unwrap()
    //drop(file);
    //drop(file_rs);
    /*
    let mut c = cc::Build::new();
    c.target("riscv64imac-unknown-none-elf");
    c.file("asm.S");
    println!("{:?}", c.get_compiler());
    c.compile("librvhasm.a");
    */
}
