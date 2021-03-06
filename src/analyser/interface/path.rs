//! TODO(liautaud):
//! Right now most of the code in this module is duplicated to handle both
//! &T and &mut T, so I should find a way to abstract this.
use analyser::interface::expressions::Output;
use analyser::interface::expressions::Wrapped;
use analyser::interface::solver::Context;
use analyser::prelude::*;
use std::fmt;
use Result;
use Tensor;

use num_traits::cast::ToPrimitive;

/// A symbolic path for a value.
#[derive(PartialEq)]
pub struct Path(Vec<isize>);

impl From<Vec<isize>> for Path {
    fn from(v: Vec<isize>) -> Path {
        Path(v)
    }
}

impl ::std::ops::Deref for Path {
    type Target = [isize];
    fn deref(&self) -> &[isize] {
        &self.0
    }
}

impl fmt::Debug for Path {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        debug_path(self, formatter)
    }
}

/// Returns the value at the given path (starting from a context).
pub fn get_path(context: &Context, path: &[isize]) -> Result<Wrapped> {
    match path[0] {
        0 => get_tensorfacts_path(&context.inputs, &path[1..]),
        1 => get_tensorfacts_path(&context.outputs, &path[1..]),
        _ => bail!(
            "The first component of path {:?} should be 0 (for the `inputs` \
             set of facts) or 1 (for the `outputs` set of facts).",
            path
        ),
    }
}

/// Sets the value at the given path (starting from a context).
pub fn set_path(context: &mut Context, path: &[isize], value: Wrapped) -> Result<()> {
    match path[0] {
        0 => set_tensorfacts_path(&mut context.inputs, &path[1..], value),
        1 => set_tensorfacts_path(&mut context.outputs, &path[1..], value),
        _ => bail!(
            "The first component of path {:?} should be 0 (for the `inputs` \
             set of facts) or 1 (for the `outputs` set of facts).",
            path
        ),
    }
}

fn debug_path(path: &[isize], formatter: &mut fmt::Formatter) -> fmt::Result {
    write!(
        formatter,
        "{}",
        match path[0] {
            0 => "inputs",
            1 => "outputs",
            _ => "buggy_path",
        }
    )?;
    debug_tensorfacts_path(&path[1..], formatter)
}

/// Returns the value at the given path (starting from a set of TensorFacts).
fn get_tensorfacts_path(facts: &Vec<TensorFact>, path: &[isize]) -> Result<Wrapped> {
    match path {
        // Get the number of facts in the set.
        [-1] => Ok(facts.len().wrap()),

        slice if slice[0] >= 0 => {
            let k = slice[0].to_usize().unwrap(); // checked

            if k < facts.len() {
                get_tensorfact_path(&facts[k], &slice[1..])
            } else {
                bail!(
                    "There are only {:?} facts in the given set, so the index \
                     {:?} is not valid.",
                    facts.len(),
                    k
                )
            }
        }

        _ => bail!(
            "The first component of subpath {:?} should either be -1 (for \
             the number of facts in the set) or a valid fact index.",
            path
        ),
    }
}

/// Sets the value at the given path (starting from a set of TensorFacts).
fn set_tensorfacts_path(facts: &mut Vec<TensorFact>, path: &[isize], value: Wrapped) -> Result<()> {
    match path {
        // Set the number of facts in the set.
        [-1] => {
            // Conversion is checked.
            let value = IntFact::from_wrapped(value)?
                .concretize()
                .map(|v| v.to_usize().unwrap());

            if value.is_some() && value.unwrap() != facts.len() {
                bail!(
                    "Can't set the length of the given set of facts to {:?} \
                     because it already has length {:?}.",
                    value,
                    facts.len()
                );
            }

            Ok(())
        }

        slice if slice[0] >= 0 => {
            // Conversion is checked.
            let k = slice[0].to_usize().unwrap();

            if k < facts.len() {
                set_tensorfact_path(&mut facts[k], &path[1..], value)
            } else {
                bail!(
                    "There are only {:?} facts in the given set, so the index \
                     {:?} is not valid.",
                    facts.len(),
                    k
                )
            }
        }

        _ => bail!(
            "The first component of subpath {:?} should either be -1 (for \
             the number of facts in the set) or a valid fact index.",
            path
        ),
    }
}

fn debug_tensorfacts_path(path: &[isize], formatter: &mut fmt::Formatter) -> fmt::Result {
    match path[0] {
        -1 => write!(formatter, ".len"),
        n => {
            write!(formatter, "[{}]", n)?;
            debug_tensorfact_path(&path[1..], formatter)
        }
    }
}

/// Returns the value at the given path (starting from a TensorFact).
fn get_tensorfact_path(fact: &TensorFact, path: &[isize]) -> Result<Wrapped> {
    match path {
        // Get the type of the TensorFact.
        [0] => Ok(fact.datatype.clone().wrap()),

        // Get the rank of the TensorFact.
        [1] => if fact.shape.open {
            Ok(IntFact::default().wrap())
        } else {
            Ok(fact.shape.dims.len().wrap())
        },

        slice if slice[0] == 2 => get_shape_path(&fact.shape, &slice[1..]),
        slice if slice[0] == 3 => get_value_path(&fact.value, &slice[1..]),

        _ => bail!(
            "The subpath {:?} should start with 0, 1, 2 or 3 (for the type, \
             rank, dimension or value of the fact respectively).",
            path
        ),
    }
}

/// Sets the value at the given path (starting from a TensorFact).
fn set_tensorfact_path(fact: &mut TensorFact, path: &[isize], value: Wrapped) -> Result<()> {
    match path {
        // Set the type of the TensorFact.
        [0] => {
            let value = TypeFact::from_wrapped(value)?;
            fact.datatype = value.unify(&fact.datatype)?;
            Ok(())
        }

        // Set the rank of the TensorFact.
        [1] => {
            if let Some(k) = IntFact::from_wrapped(value)?.concretize() {
                if k >= 0 {
                    let k = k.to_usize().unwrap();
                    fact.shape = fact.shape.unify(&ShapeFact::closed(vec![dimfact!(_); k]))?;
                } else {
                    bail!("Infered a negative rank ({})", k)
                }
            }

            Ok(())
        }

        // Set the whole shape of the TensorFact.
        [2] => {
            let shape = ShapeFact::from_wrapped(value)?;
            fact.shape = shape.unify(&fact.shape)?;

            Ok(())
        }

        // Set a precise dimension of the TensorFact.
        [2, k] => {
            let k = k.to_usize().unwrap();
            let dim = DimFact::from_wrapped(value)?;

            let mut dims = vec![dimfact!(_); k];
            dims.push(dim);

            fact.shape = fact.shape.unify(&ShapeFact::open(dims))?;

            Ok(())
        }

        // Set full TensorFact value, also unifying type and shape.
        [3] => {
            let value = ValueFact::from_wrapped(value)?;
            fact.value = fact.value.unify(&value)?;
            if let Some(tensor) = fact.value.concretize() {
                fact.shape = fact.shape.unify(&ShapeFact::from(tensor.shape()))?;
                fact.datatype = fact.datatype.unify(&TypeFact::from(tensor.datatype()))?;
            }
            Ok(())
        }

        slice if slice[0] == 3 => {
            debug!("FIXME Unimplemented set_value_path for individual value");
            Ok(())
        }

        _ => bail!(
            "The subpath {:?} should start with 0, 1, 2 or 3 (for the type, \
             rank, dimension or value of the fact respectively).",
            path
        ),
    }
}

fn debug_tensorfact_path(path: &[isize], formatter: &mut fmt::Formatter) -> fmt::Result {
    match path {
        [] => Ok(()),
        [0] => write!(formatter, ".datatype"),
        [1] => write!(formatter, ".rank"),
        [2] => write!(formatter, ".shape"),
        [2, k] => write!(formatter, ".shape[{}]", k),
        slice if slice[0] == 3 => debug_value_path(&path[1..], formatter),
        _ => write!(formatter, ".invalid"),
    }
}

/// Returns the shape or dimension at the given path (starting from a ShapeFact).
fn get_shape_path(shape: &ShapeFact, path: &[isize]) -> Result<Wrapped> {
    match path {
        // Get the whole shape.
        [] => Ok(shape.clone().wrap()),

        // Get a precise dimension.
        [k] => {
            let k = k.to_usize().unwrap();

            if k < shape.dims.len() {
                Ok(shape.dims[k].wrap())
            } else if shape.open {
                Ok(dimfact!(_).wrap())
            } else {
                bail!(
                    "The closed shape {:?} has no {:?}-th dimension.",
                    shape.dims,
                    k
                );
            }
        }

        _ => bail!(
            "The subpath {:?} for the shape should either be [] (for the \
             entire shape) or [k] with k the index of a dimension.",
            path
        ),
    }
}

/// Returns the value at the given path (starting from a ValueFact).
fn get_value_path(value: &ValueFact, path: &[isize]) -> Result<Wrapped> {
    trace!("get_value_path path:{:?} value:{:?}", path, value);
    // Return the whole tensor.
    if path == &[-1] || path == &[] {
        return Ok(value.clone().wrap());
    }

    let path: Vec<_> = path.iter().map(|i| i.to_usize().unwrap()).collect();

    macro_rules! inner {
        ($array:expr) => {{
            match $array.get(path.as_slice()) {
                Some(&v) => Ok((v as isize).wrap()),
                None => bail!("There is no index {:?} in value {:?}.", path, $array),
            }
        }};
    };

    match value.concretize() {
        None => Ok(IntFact::default().wrap()),
        Some(tensor) => match tensor {
            Tensor::I32(array) => inner!(array),
            Tensor::I8(array) => inner!(array),
            Tensor::U8(array) => inner!(array),
            _ => bail!(
                "Found value {:?}, but the solver only supports \
                 integer values.",
                tensor
            ),
        },
    }
}

fn debug_value_path(path: &[isize], formatter: &mut fmt::Formatter) -> fmt::Result {
    for p in path {
        write!(formatter, "[{}]", p)?;
    }
    Ok(())
}
