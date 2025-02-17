// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::display::{display_optional, SliceDisplayer};
use crate::ts_type::{ts_type_ann_to_def, TsTypeDef};
use deno_ast::swc::ast::{ObjectPatProp, Pat, TsFnParam};
use deno_ast::ParsedSource;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "kind")]
pub enum ParamDef {
  #[serde(rename_all = "camelCase")]
  Array {
    elements: Vec<Option<ParamDef>>,
    optional: bool,
    ts_type: Option<TsTypeDef>,
  },
  #[serde(rename_all = "camelCase")]
  Assign {
    left: Box<ParamDef>,
    right: String,
    ts_type: Option<TsTypeDef>,
  },
  #[serde(rename_all = "camelCase")]
  Identifier {
    name: String,
    optional: bool,
    ts_type: Option<TsTypeDef>,
  },
  #[serde(rename_all = "camelCase")]
  Object {
    props: Vec<ObjectPatPropDef>,
    optional: bool,
    ts_type: Option<TsTypeDef>,
  },
  #[serde(rename_all = "camelCase")]
  Rest {
    arg: Box<ParamDef>,
    ts_type: Option<TsTypeDef>,
  },
}

impl Display for ParamDef {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    match self {
      ParamDef::Array {
        elements,
        optional,
        ts_type,
      } => {
        write!(f, "[")?;
        if !elements.is_empty() {
          if let Some(v) = &elements[0] {
            write!(f, "{}", v)?;
          }
          for maybe_v in &elements[1..] {
            write!(f, ", ")?;
            if let Some(v) = maybe_v {
              write!(f, "{}", v)?;
            }
          }
        }
        write!(f, "]")?;
        write!(f, "{}", display_optional(*optional))?;
        if let Some(ts_type) = ts_type {
          write!(f, ": {}", ts_type)?;
        }
        Ok(())
      }
      ParamDef::Assign { left, ts_type, .. } => {
        write!(f, "{}", left)?;
        if let Some(ts_type) = ts_type {
          write!(f, ": {}", ts_type)?;
        }
        // TODO(SyrupThinker) As we cannot display expressions the value is just omitted
        // write!(f, " = {}", right)?;
        Ok(())
      }
      ParamDef::Identifier {
        name,
        optional,
        ts_type,
      } => {
        write!(f, "{}{}", name, display_optional(*optional))?;
        if let Some(ts_type) = ts_type {
          write!(f, ": {}", ts_type)?;
        }
        Ok(())
      }
      ParamDef::Object {
        props,
        optional,
        ts_type,
      } => {
        write!(
          f,
          "{{{}}}{}",
          SliceDisplayer::new(props, ", ", false),
          display_optional(*optional)
        )?;
        if let Some(ts_type) = ts_type {
          write!(f, ": {}", ts_type)?;
        }
        Ok(())
      }
      ParamDef::Rest { arg, ts_type } => {
        write!(f, "...{}", arg)?;
        if let Some(ts_type) = ts_type {
          write!(f, ": {}", ts_type)?;
        }
        Ok(())
      }
    }
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "kind")]
pub enum ObjectPatPropDef {
  Assign { key: String, value: Option<String> },
  KeyValue { key: String, value: Box<ParamDef> },
  Rest { arg: Box<ParamDef> },
}

impl Display for ObjectPatPropDef {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    match self {
      ObjectPatPropDef::KeyValue { key, .. } => {
        // The internal identifier does not need to be exposed
        write!(f, "{}", key)
      }
      ObjectPatPropDef::Assign { key, value } => {
        if let Some(_value) = value {
          // TODO(SyrupThinker) As we cannot display expressions the value is just omitted
          write!(f, "{}", key)
        } else {
          write!(f, "{}", key)
        }
      }
      ObjectPatPropDef::Rest { arg } => write!(f, "...{}", arg),
    }
  }
}

pub fn ident_to_param_def(
  _parsed_source: Option<&ParsedSource>,
  ident: &deno_ast::swc::ast::BindingIdent,
) -> ParamDef {
  let ts_type = ident.type_ann.as_ref().map(|rt| ts_type_ann_to_def(rt));

  ParamDef::Identifier {
    name: ident.id.sym.to_string(),
    optional: ident.id.optional,
    ts_type,
  }
}

fn rest_pat_to_param_def(
  parsed_source: Option<&ParsedSource>,
  rest_pat: &deno_ast::swc::ast::RestPat,
) -> ParamDef {
  let ts_type = rest_pat.type_ann.as_ref().map(|rt| ts_type_ann_to_def(rt));

  ParamDef::Rest {
    arg: Box::new(pat_to_param_def(parsed_source, &*rest_pat.arg)),
    ts_type,
  }
}

fn object_pat_prop_to_def(
  parsed_source: Option<&ParsedSource>,
  object_pat_prop: &ObjectPatProp,
) -> ObjectPatPropDef {
  match object_pat_prop {
    ObjectPatProp::Assign(assign) => ObjectPatPropDef::Assign {
      key: assign.key.sym.to_string(),
      value: assign.value.as_ref().map(|_| "[UNSUPPORTED]".to_string()),
    },
    ObjectPatProp::KeyValue(keyvalue) => ObjectPatPropDef::KeyValue {
      key: prop_name_to_string(parsed_source, &keyvalue.key),
      value: Box::new(pat_to_param_def(parsed_source, &*keyvalue.value)),
    },
    ObjectPatProp::Rest(rest) => ObjectPatPropDef::Rest {
      arg: Box::new(pat_to_param_def(parsed_source, &*rest.arg)),
    },
  }
}

fn object_pat_to_param_def(
  parsed_source: Option<&ParsedSource>,
  object_pat: &deno_ast::swc::ast::ObjectPat,
) -> ParamDef {
  let props = object_pat
    .props
    .iter()
    .map(|prop| object_pat_prop_to_def(parsed_source, prop))
    .collect::<Vec<_>>();
  let ts_type = object_pat
    .type_ann
    .as_ref()
    .map(|rt| ts_type_ann_to_def(rt));

  ParamDef::Object {
    props,
    optional: object_pat.optional,
    ts_type,
  }
}

fn array_pat_to_param_def(
  parsed_source: Option<&ParsedSource>,
  array_pat: &deno_ast::swc::ast::ArrayPat,
) -> ParamDef {
  let elements = array_pat
    .elems
    .iter()
    .map(|elem| elem.as_ref().map(|e| pat_to_param_def(parsed_source, e)))
    .collect::<Vec<Option<_>>>();
  let ts_type = array_pat.type_ann.as_ref().map(|rt| ts_type_ann_to_def(rt));

  ParamDef::Array {
    elements,
    optional: array_pat.optional,
    ts_type,
  }
}

pub fn assign_pat_to_param_def(
  parsed_source: Option<&ParsedSource>,
  assign_pat: &deno_ast::swc::ast::AssignPat,
) -> ParamDef {
  let ts_type = assign_pat
    .type_ann
    .as_ref()
    .map(|rt| ts_type_ann_to_def(rt));

  ParamDef::Assign {
    left: Box::new(pat_to_param_def(parsed_source, &*assign_pat.left)),
    right: "[UNSUPPORTED]".to_string(),
    ts_type,
  }
}

pub fn pat_to_param_def(
  parsed_source: Option<&ParsedSource>,
  pat: &deno_ast::swc::ast::Pat,
) -> ParamDef {
  match pat {
    Pat::Ident(ident) => ident_to_param_def(parsed_source, ident),
    Pat::Array(array_pat) => array_pat_to_param_def(parsed_source, array_pat),
    Pat::Rest(rest_pat) => rest_pat_to_param_def(parsed_source, rest_pat),
    Pat::Object(object_pat) => {
      object_pat_to_param_def(parsed_source, object_pat)
    }
    Pat::Assign(assign_pat) => {
      assign_pat_to_param_def(parsed_source, assign_pat)
    }
    _ => unreachable!(),
  }
}

pub fn ts_fn_param_to_param_def(
  parsed_source: Option<&ParsedSource>,
  ts_fn_param: &deno_ast::swc::ast::TsFnParam,
) -> ParamDef {
  match ts_fn_param {
    TsFnParam::Ident(ident) => ident_to_param_def(parsed_source, ident),
    TsFnParam::Array(array_pat) => {
      array_pat_to_param_def(parsed_source, array_pat)
    }
    TsFnParam::Rest(rest_pat) => rest_pat_to_param_def(parsed_source, rest_pat),
    TsFnParam::Object(object_pat) => {
      object_pat_to_param_def(parsed_source, object_pat)
    }
  }
}

pub fn prop_name_to_string(
  parsed_source: Option<&ParsedSource>,
  prop_name: &deno_ast::swc::ast::PropName,
) -> String {
  use deno_ast::swc::ast::PropName;
  match prop_name {
    PropName::Ident(ident) => ident.sym.to_string(),
    PropName::Str(str_) => str_.value.to_string(),
    PropName::Num(num) => num.value.to_string(),
    PropName::BigInt(num) => num.value.to_string(),
    PropName::Computed(comp_prop_name) => parsed_source
      .map(|s| s.source().span_text(&comp_prop_name.span).to_string())
      .unwrap_or_else(|| "<UNAVAILABLE>".to_string()),
  }
}
