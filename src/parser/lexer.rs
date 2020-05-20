/*
 * ******************************************************************************************
 * Copyright (c) 2019 Pascal Kuthe. This file is part of the OpenVAF project.
 * It is subject to the license terms in the LICENSE file found in the top-level directory
 *  of this distribution and at  https://gitlab.com/DSPOM/OpenVAF/blob/master/LICENSE.
 *  No part of OpenVAF, including this file, may be copied, modified, propagated, or
 *  distributed except according to the terms contained in the LICENSE file.
 * *****************************************************************************************
 */

use logos::internal::LexerInternal;
use logos::Logos;

use crate::span::{Index, LineNumber, Range};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, PartialEq, Copy, Eq)]
pub struct FollowedByBracket(pub bool);

//in terms of api this just serves as a lexer token enum. however it actually is the real lexer generated by logos.
#[derive(Clone, Logos, Debug, PartialEq, Copy, Eq)]
pub enum Token {
    //Newline handling
    #[token("\\\n")]
    MacroDefNewLine,

    #[token("\n")]
    Newline,

    #[regex(r"//[^\n]*\n", single_line_comment)]
    #[token("/*", ignore_multiline_comment)]
    Comment(LineNumber),

    //Mock tokens only used for error reporting
    CommentEnd,
    EOF,

    //Actual Tokens

    //required rules
    #[regex(r"[ \t\f]+", logos::skip)]
    #[error]
    Unexpected,

    UnexpectedEOF,

    #[regex(r"`[a-zA-Z_][a-zA-Z_0-9\$]*")]
    MacroReference,
    //Compiler directives
    #[token("`include")]
    Include,
    #[token("`ifdef")]
    MacroIf,
    #[token("`ifndef")]
    MacroIfn,
    #[token("`elsif")]
    MacroElsif,
    #[token("`else")]
    MacroElse,
    #[token("`endif")]
    MacroEndIf,
    #[token("`define")]
    MacroDef,

    //Identifiers
    #[regex(r"[a-zA-Z_][[:word:]\$]*", handle_simple_ident)]
    SimpleIdentifier(FollowedByBracket),
    #[regex(r"\\[[:print:]&&\S]+\s")]
    EscapedIdentifier,

    #[regex(r"\$[a-zA-Z0-9_\$][a-zA-Z0-9_\$]*")]
    SystemCall,
    #[token("$temperature")]
    Temperature,
    #[token("$vt")]
    Vt,
    #[token("$simparam")]
    SimParam,
    #[token("$simparam$str")]
    SimParamStr,
    #[token("$port_connected")]
    PortConnected,
    #[token("$param_given")]
    ParamGiven,

    //Constants
    #[regex(r#""([^\n"\\]|\\[\\tn")])*""#)]
    LiteralString,

    #[regex(r"[0-9][0-9_]*")]
    LiteralUnsignedNumber,
    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*[TGMKkmupfa]")]
    LiteralRealNumberDotScaleChar,
    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*")]
    LiteralRealNumberDotExp,
    #[regex(r"[0-9][0-9_]*[TGMKkmupfa]")]
    LiteralRealNumberScaleChar,
    #[regex(r"[0-9][0-9_]*[eE][+-]?[0-9][0-9_]*")]
    LiteralRealNumberExp,
    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*")]
    LiteralRealNumberDot,

    //Symbols
    #[token(".")]
    Accessor,
    #[token(";")]
    Semicolon,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,
    #[token("(")]
    ParenOpen,
    #[token(")")]
    ParenClose,
    #[token("(*")]
    AttributeStart,
    #[token("*)")]
    AttributeEnd,
    #[token("[")]
    SquareBracketOpen,
    #[token("]")]
    SquareBracketClose,
    #[token("<+")]
    Contribute,
    #[token("=")]
    Assign,
    #[token("#")]
    Hash,

    //Arithmatic Operators
    #[token("*")]
    OpMul,
    #[token("/")]
    OpDiv,
    #[token("%")]
    OpModulus,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("**")]
    OpExp,
    //UnaryOperators
    #[token("!")]
    OpLogicNot,
    #[token("~")]
    OpBitNot,

    #[token("<<")]
    OpArithmeticShiftLeft,
    #[token(">>")]
    OpArithmeticShiftRight,

    //Relational
    #[token("<")]
    OpLess,
    #[token("<=")]
    OpLessEqual,
    #[token(">")]
    OpGreater,
    #[token(">=")]
    OpGreaterEqual,
    #[token("==")]
    OpEqual,
    #[token("!=")]
    OpNotEqual,
    //Logic
    #[token("&&")]
    OpLogicAnd,
    #[token("||")]
    OpLogicalOr,

    //Bit
    #[token("&")]
    OpBitAnd,
    #[token("^")]
    OpBitXor,
    #[token("~^")]
    #[token("^~")]
    OpBitNXor,
    #[token("|")]
    OpBitOr,

    //Other
    #[token("?")]
    OpCondition,

    //Keywords
    #[token("if")]
    If,
    #[token("else")]
    Else,

    #[token("while")]
    While,

    #[token("begin")]
    Begin,
    #[token("end")]
    End,

    #[token("module")]
    Module,
    #[token("endmodule")]
    EndModule,
    #[token("discipline")]
    Discipline,
    #[token("enddiscipline")]
    EndDiscipline,

    #[token("nature")]
    Nature,
    #[token("endnature")]
    EndNature,

    #[token("branch")]
    Branch,
    #[token("parameter")]
    Parameter,
    #[token("localparam")]
    DefineParameter,
    #[token("defparam")]
    LocalParameter,

    #[token("analog")]
    Analog,
    #[token("initial")]
    AnalogInitial,

    #[token("input")]
    Input,
    #[token("inout")]
    Inout,
    #[token("output")]
    Output,

    #[token("signed")]
    Signed,
    #[token("vectored")]
    Vectored,
    #[token("scalared")]
    Scalared,

    //Types
    #[token("string")]
    String,
    #[token("time")]
    Time,
    #[token("realtime")]
    Realtime,
    #[token("integer")]
    Integer,
    #[token("real")]
    Real,
    #[token("reg")]
    Reg,
    #[token("wreal")]
    Wreal,
    #[token("supply0")]
    Supply0,
    #[token("supply1")]
    Supply1,
    #[token("tri")]
    Tri,
    #[token("triand")]
    TriAnd,
    #[token("trior")]
    TriOr,
    #[token("tri0")]
    Tri0,
    #[token("tri1")]
    Tri1,
    #[token("wire")]
    Wire,
    #[token("uwire")]
    Uwire,
    #[token("wand")]
    Wand,
    #[token("wor")]
    Wor,
    #[token("ground")]
    Ground,

    #[token("potential")]
    Potential,
    #[token("flow")]
    Flow,
    #[token("domain")]
    Domain,
    #[token("discrete")]
    Discrete,
    #[token("continuous")]
    Continuous,

    #[token("ddt")]
    TimeDerivative,
    #[token("ddx")]
    PartialDerivative,
    #[token("idt")]
    TimeIntegral,
    #[token("idtmod")]
    TimeIntegralMod,
    #[token("limexp")]
    LimExp,
    #[token("white_noise")]
    WhiteNoise,
    #[token("flicker_noise")]
    FlickerNoise,

    #[token("$pow")]
    #[token("pow")]
    Pow,
    #[token("$sqrt")]
    #[token("sqrt")]
    Sqrt,

    #[token("$hypot")]
    #[token("hypot")]
    Hypot,
    #[token("$exp")]
    #[token("exp")]
    Exp,
    #[token("$ln")]
    #[token("ln")]
    Ln,
    #[token("$log10")]
    #[token("log")]
    Log,
    #[token("$min")]
    #[token("min")]
    Min,
    #[token("$max")]
    #[token("max")]
    Max,
    #[token("$abs")]
    #[token("abs")]
    Abs,
    #[token("$floor")]
    #[token("floor")]
    Floor,
    #[token("$ceil")]
    #[token("ceil")]
    Ceil,

    #[token("$sin")]
    #[token("sin")]
    Sin,
    #[token("$cos")]
    #[token("cos")]
    Cos,
    #[token("tan")]
    #[token("$tan")]
    Tan,

    #[token("$asin")]
    #[token("asin")]
    ArcSin,
    #[token("$acos")]
    #[token("acos")]
    ArcCos,
    #[token("atan")]
    #[token("$atan")]
    ArcTan,
    #[token("atan2")]
    #[token("$atan2")]
    ArcTan2,

    #[token("sinh")]
    #[token("$sinh")]
    SinH,
    #[token("cosh")]
    #[token("$cosh")]
    CosH,
    #[token("tanh")]
    #[token("$tanh")]
    TanH,

    #[token("asinh")]
    #[token("$asinh")]
    ArcSinH,
    #[token("acosh")]
    #[token("$acosh")]
    ArcCosH,
    #[token("atanh")]
    #[token("$atanh")]
    ArcTanH,

    #[token("from")]
    From,
    #[token("exclude")]
    Exclude,
    #[token("inf")]
    Infinity,
    #[token("-inf")]
    MinusInfinity,

    #[token("abstol")]
    Abstol,
    #[token("access")]
    Access,
    #[token("ddt_nature")]
    TimeDerivativeNature,
    #[token("idt_nature")]
    TimeIntegralNature,
    #[token("units")]
    Units,
}

#[inline(always)]
fn single_line_comment<'source>(_: &mut logos::Lexer<'source, Token>) -> LineNumber {
    1
}

#[inline]
fn ignore_multiline_comment<'source>(lex: &mut logos::Lexer<'source, Token>) -> Option<LineNumber> {
    let mut lines: LineNumber = 0;
    loop {
        match lex.read()? {
            b'*' => {
                lex.bump(1);
                if lex.read() == Some(b'/') {
                    lex.bump(1);
                    break;
                }
            }
            b'\n' => {
                lines += 1;
                lex.bump(1)
            }
            _ => lex.bump(1),
        }
    }
    Some(lines)
}
#[inline]
fn handle_simple_ident<'source>(lex: &mut logos::Lexer<'source, Token>) -> FollowedByBracket {
    FollowedByBracket(lex.read() == Some(b'('))
}

pub struct Lexer<'lt> {
    internal: logos::Lexer<'lt, Token>,
}
impl<'lt> Lexer<'lt> {
    pub fn new(source: &'lt str) -> Self {
        Self {
            internal: Token::lexer(source),
        }
    }

    #[cfg(test)]
    pub fn new_test(source: &'lt str) -> Self {
        let mut res = Self {
            internal: Token::lexer(source),
        };
        res
    }

    pub fn peek(&self) -> (Range, Option<Token>) {
        let mut lexer = self.internal.clone();
        let token = lexer.next();
        let range = lexer.span();
        let range = Range {
            start: range.start as Index,
            end: range.end as Index,
        };
        (range, token)
    }
    #[cfg(test)]
    pub fn test_next(&mut self) -> Option<Token> {
        loop {
            match self.internal.next() {
                Some(Token::Newline) | Some(Token::Comment(_)) => (),
                res => return res,
            }
        }
    }

    pub fn range(&self) -> Range {
        let internal_range = self.internal.span();
        Range {
            start: internal_range.start as Index,
            end: internal_range.end as Index,
        }
    }
    pub fn token_len(&self) -> Index {
        self.range().end - self.range().start
    }

    pub fn slice(&self) -> &str {
        self.internal.slice()
    }
}
impl<'source> Iterator for Lexer<'source> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.internal.next()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn macro_if() {
        let mut lexer = Lexer::new("`ifdef");
        assert_eq!(lexer.next(), Some(Token::MacroIf));
    }
    #[test]
    pub fn macro_ifn() {
        let mut lexer = Lexer::new("`ifndef");
        assert_eq!(lexer.next(), Some(Token::MacroIfn));
    }
    #[test]
    pub fn macro_else() {
        let mut lexer = Lexer::new("`else");
        assert_eq!(lexer.next(), Some(Token::MacroElse));
    }
    #[test]
    pub fn macro_elsif() {
        let mut lexer = Lexer::new("`elsif");
        assert_eq!(lexer.next(), Some(Token::MacroElsif));
    }
    #[test]
    pub fn macro_definition() {
        let mut lexer = Lexer::new("`define x(y) \\\n test");
        assert_eq!(lexer.test_next(), Some(Token::MacroDef));
        assert_eq!(
            lexer.test_next(),
            Some(Token::SimpleIdentifier(FollowedByBracket(true)))
        );
        assert_eq!(lexer.test_next(), Some(Token::ParenOpen));
        assert_eq!(
            lexer.test_next(),
            Some(Token::SimpleIdentifier(FollowedByBracket(false)))
        );
        assert_eq!(lexer.test_next(), Some(Token::ParenClose));
        assert_eq!(lexer.test_next(), Some(Token::MacroDefNewLine));
        assert_eq!(
            lexer.test_next(),
            Some(Token::SimpleIdentifier(FollowedByBracket(false)))
        );
    }
    #[test]
    pub fn include() {
        assert_eq!(Lexer::new("`include").next(), Some(Token::Include));
    }
    #[test]
    pub fn simple_ident() {
        let mut lexer = Lexer::new_test("test _test  egta  test$\ntest2_$ iftest");
        assert_eq!(
            lexer.test_next(),
            Some(Token::SimpleIdentifier(FollowedByBracket(false)))
        );
        assert_eq!(lexer.slice(), "test");
        assert_eq!(
            lexer.test_next(),
            Some(Token::SimpleIdentifier(FollowedByBracket(false)))
        );
        assert_eq!(lexer.slice(), "_test");
        assert_eq!(
            lexer.test_next(),
            Some(Token::SimpleIdentifier(FollowedByBracket(false)))
        );
        assert_eq!(lexer.slice(), "egta");
        assert_eq!(
            lexer.test_next(),
            Some(Token::SimpleIdentifier(FollowedByBracket(false)))
        );
        assert_eq!(lexer.slice(), "test$");
        assert_eq!(
            lexer.test_next(),
            Some(Token::SimpleIdentifier(FollowedByBracket(false)))
        );
        assert_eq!(lexer.slice(), "test2_$");
        assert_eq!(
            lexer.test_next(),
            Some(Token::SimpleIdentifier(FollowedByBracket(false)))
        );
        assert_eq!(lexer.slice(), "iftest");
    }
    #[test]
    pub fn escaped_ident() {
        let mut lexer = Lexer::new("\\lel\\\\lel \\if ");
        assert_eq!(lexer.test_next(), Some(Token::EscapedIdentifier));
        assert_eq!(&lexer.slice()[1..9], "lel\\\\lel");
        assert_eq!(lexer.test_next(), Some(Token::EscapedIdentifier));
        assert_eq!(&lexer.slice()[1..3], "if");
    }
    #[test]
    pub fn comment() {
        let mut lexer = Lexer::new_test("//jdfjdfjw4$%\r%&/**#.,|\ntest");
        assert_eq!(
            lexer.test_next(),
            Some(Token::SimpleIdentifier(FollowedByBracket(false)))
        );
        assert_eq!(lexer.slice(), "test")
    }
    #[test]
    pub fn block_comment() {
        let mut lexer = Lexer::new_test("/*A\nB\n*C*/`test");
        assert_eq!(lexer.test_next(), Some(Token::MacroReference));
        assert_eq!(lexer.slice(), "`test")
    }
    #[test]
    pub fn string() {
        let mut lexer = Lexer::new(r#""lel\"dsd%§.,-032391\t    ""#);
        assert_eq!(lexer.test_next(), Some(Token::LiteralString));
    }
    #[test]
    pub fn unsigned_number() {
        let mut lexer = Lexer::new("1_2345_5678_9");
        assert_eq!(lexer.test_next(), Some(Token::LiteralUnsignedNumber));
    }
    #[test]
    pub fn macro_ref() {
        let test = "`egta";

        let mut lexer = Lexer::new_test(test);
        assert_eq!(lexer.test_next(), Some(Token::MacroReference))
    }
    #[test]
    pub fn real_number() {
        let mut lexer = Lexer::new_test(
            "1.2
            0.1
            2394.26331
            1.2E12 // the exponent symbol can be e or E
            1.30e-2
            0.1e-0
            236.123_763_e-12 // underscores are ignored
            1.3u
            23E10
            29E-2
            7k",
        );
        assert_eq!(lexer.test_next(), Some(Token::LiteralRealNumberDot));
        assert_eq!(lexer.test_next(), Some(Token::LiteralRealNumberDot));
        assert_eq!(lexer.test_next(), Some(Token::LiteralRealNumberDot));
        assert_eq!(lexer.test_next(), Some(Token::LiteralRealNumberDotExp));
        assert_eq!(lexer.test_next(), Some(Token::LiteralRealNumberDotExp));
        assert_eq!(lexer.test_next(), Some(Token::LiteralRealNumberDotExp));
        assert_eq!(lexer.test_next(), Some(Token::LiteralRealNumberDotExp));
        assert_eq!(
            lexer.test_next(),
            Some(Token::LiteralRealNumberDotScaleChar)
        );
        assert_eq!(lexer.test_next(), Some(Token::LiteralRealNumberExp));
        assert_eq!(lexer.test_next(), Some(Token::LiteralRealNumberExp));
        assert_eq!(lexer.test_next(), Some(Token::LiteralRealNumberScaleChar));
    }
}
// The following is used for error messeges

impl Display for Token {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Token::MacroDefNewLine => f.write_str("\\ (newline)"),
            Token::Newline => f.write_str("(newline)"),
            Token::Comment(_) => f.write_str("comment"),
            Token::CommentEnd => f.write_str("*/"),
            Token::EOF => f.write_str("Source file end"),
            Token::Unexpected => f.write_str("unexpected Sequence"),
            Token::UnexpectedEOF => f.write_str("unexpected EOF"),
            Token::MacroReference => f.write_str("macro reference"),
            Token::Include => f.write_str("`include"),
            Token::MacroIf => f.write_str("`if"),
            Token::MacroIfn => f.write_str("`ifn"),
            Token::MacroElsif => f.write_str("`elsif"),
            Token::MacroElse => f.write_str("`else"),
            Token::MacroEndIf => f.write_str("`end"),
            Token::MacroDef => f.write_str("`define"),
            Token::SimpleIdentifier(_) => f.write_str("simple identifier"),
            Token::EscapedIdentifier => f.write_str("escaped identifier"),
            Token::SystemCall => f.write_str("system call"),
            Token::LiteralString => f.write_str("string literal"),
            Token::LiteralUnsignedNumber => f.write_str("unsigned number"),
            Token::LiteralRealNumberDotScaleChar => f.write_str("real number"),
            Token::LiteralRealNumberDotExp => f.write_str("real number"),
            Token::LiteralRealNumberScaleChar => f.write_str("real number"),
            Token::LiteralRealNumberExp => f.write_str("real number"),
            Token::LiteralRealNumberDot => f.write_str("real number"),
            Token::Accessor => f.write_str("."),
            Token::Semicolon => f.write_str(";"),
            Token::Colon => f.write_str(":"),
            Token::Comma => f.write_str(","),
            Token::ParenOpen => f.write_str("("),
            Token::ParenClose => f.write_str(")"),
            Token::AttributeStart => f.write_str("(*"),
            Token::AttributeEnd => f.write_str("*)"),
            Token::SquareBracketOpen => f.write_str("["),
            Token::SquareBracketClose => f.write_str("]"),
            Token::Contribute => f.write_str("<+"),
            Token::Assign => f.write_str("="),
            Token::Hash => f.write_str("#"),
            Token::OpMul => f.write_str("*"),
            Token::OpDiv => f.write_str(""),
            Token::OpModulus => f.write_str(""),
            Token::Plus => f.write_str("%"),
            Token::Minus => f.write_str("-"),
            Token::OpExp => f.write_str("exp"),
            Token::OpLogicNot => f.write_str("!"),
            Token::OpBitNot => f.write_str("~"),
            Token::OpArithmeticShiftLeft => f.write_str("<<"),
            Token::OpArithmeticShiftRight => f.write_str(">>"),
            Token::OpLess => f.write_str("<"),
            Token::OpLessEqual => f.write_str("<="),
            Token::OpGreater => f.write_str(">"),
            Token::OpGreaterEqual => f.write_str(">="),
            Token::OpEqual => f.write_str("=="),
            Token::OpNotEqual => f.write_str("!="),
            Token::OpLogicAnd => f.write_str("&&"),
            Token::OpLogicalOr => f.write_str("||"),
            Token::OpBitAnd => f.write_str("&"),
            Token::OpBitXor => f.write_str("^"),
            Token::OpBitNXor => f.write_str("^~/~^"),
            Token::OpBitOr => f.write_str("|"),
            Token::OpCondition => f.write_str("?"),
            Token::If => f.write_str("if"),
            Token::Else => f.write_str("else"),
            Token::While => f.write_str("while"),
            Token::Begin => f.write_str("begin"),
            Token::End => f.write_str("end"),
            Token::Module => f.write_str("module"),
            Token::EndModule => f.write_str("endmodule"),
            Token::Discipline => f.write_str("discipline"),
            Token::EndDiscipline => f.write_str("enddiscipline"),
            Token::Nature => f.write_str("nature"),
            Token::EndNature => f.write_str("endnature"),
            Token::Branch => f.write_str("branch"),
            Token::Parameter => f.write_str("parameter"),
            Token::DefineParameter => f.write_str("defparam"),
            Token::LocalParameter => f.write_str("localparam"),
            Token::Analog => f.write_str("analog"),
            Token::AnalogInitial => f.write_str("inital"),
            Token::Input => f.write_str("input"),
            Token::Inout => f.write_str("inout"),
            Token::Output => f.write_str("output"),
            Token::Signed => f.write_str("signed"),
            Token::Vectored => f.write_str("vectored"),
            Token::Scalared => f.write_str("scalared"),
            Token::String => f.write_str("string"),
            Token::Time => f.write_str("time"),
            Token::Realtime => f.write_str("realtime"),
            Token::Integer => f.write_str("integer"),
            Token::Real => f.write_str("real"),
            Token::Reg => f.write_str("reg"),
            Token::Wreal => f.write_str("wreal"),
            Token::Supply0 => f.write_str("supply0"),
            Token::Supply1 => f.write_str("supply1"),
            Token::Tri => f.write_str("tri"),
            Token::TriAnd => f.write_str("triand"),
            Token::TriOr => f.write_str("trior"),
            Token::Tri0 => f.write_str("tri0"),
            Token::Tri1 => f.write_str("tri1"),
            Token::Wire => f.write_str("wire"),
            Token::Uwire => f.write_str("uwire"),
            Token::Wand => f.write_str("wand"),
            Token::Wor => f.write_str("wor"),
            Token::Ground => f.write_str("ground"),
            Token::Potential => f.write_str("potential"),
            Token::Flow => f.write_str("flow"),
            Token::Domain => f.write_str("domain"),
            Token::Discrete => f.write_str("discrete"),
            Token::Continuous => f.write_str("continuous"),
            Token::TimeDerivative => f.write_str("ddt"),
            Token::PartialDerivative => f.write_str("ddx"),
            Token::TimeIntegral => f.write_str("idt"),
            Token::TimeIntegralMod => f.write_str("idtmod"),
            Token::LimExp => f.write_str("limexp"),
            Token::WhiteNoise => f.write_str("whitenoise"),
            Token::FlickerNoise => f.write_str("flickrnoise"),
            Token::Pow => f.write_str("pow"),
            Token::Sqrt => f.write_str("sqrt"),
            Token::Hypot => f.write_str("hypot"),
            Token::Exp => f.write_str("exp"),
            Token::Ln => f.write_str("ln"),
            Token::Log => f.write_str("log"),
            Token::Min => f.write_str("min"),
            Token::Max => f.write_str("max"),
            Token::Abs => f.write_str("abs"),
            Token::Floor => f.write_str("floor"),
            Token::Ceil => f.write_str("ceil"),
            Token::Sin => f.write_str("sin"),
            Token::Cos => f.write_str("cos"),
            Token::Tan => f.write_str("tan"),
            Token::ArcSin => f.write_str("asin"),
            Token::ArcCos => f.write_str("acos"),
            Token::ArcTan => f.write_str("atan"),
            Token::ArcTan2 => f.write_str("atan2"),
            Token::SinH => f.write_str("sinh"),
            Token::CosH => f.write_str("cosh"),
            Token::TanH => f.write_str("tanh"),
            Token::ArcSinH => f.write_str("asinh"),
            Token::ArcCosH => f.write_str("acosh"),
            Token::ArcTanH => f.write_str("atanh"),
            Token::From => f.write_str("from"),
            Token::Exclude => f.write_str("exclude"),
            Token::Infinity => f.write_str("inf"),
            Token::MinusInfinity => f.write_str("-inf"),
            Token::Abstol => f.write_str("abstol"),
            Token::Access => f.write_str("access"),
            Token::TimeDerivativeNature => f.write_str("ddt_nature"),
            Token::TimeIntegralNature => f.write_str("idt_nature"),
            Token::Units => f.write_str("units"),
            Token::Temperature => f.write_str("$temperature"),
            Token::Vt => f.write_str("$vt"),
            Token::SimParam => f.write_str("$simparam"),
            Token::SimParamStr => f.write_str("$simparam$str"),
            Token::PortConnected => f.write_str("$port_connected"),
            Token::ParamGiven => f.write_str("$param_given"),
        }
    }
}
