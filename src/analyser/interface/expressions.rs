use std::marker::PhantomData;
use std::fmt::Debug;

use num_traits::Num;
use num_traits::CheckedDiv;

use Result;
use tfpb::types::DataType;
use analyser::types::DimFact;
use analyser::interface::path::Path;
use analyser::interface::solver::Context;
use analyser::interface::proxies::IntProxy;
use analyser::interface::proxies::DimProxy;
use analyser::interface::proxies::TypeProxy;


/// The types of values that expressions can produce.
pub trait Datum: Debug + Clone + Copy + PartialEq {
    fn into_wrapped(source: Self) -> Wrapped;
    fn from_wrapped(wrapped: Wrapped) -> Self;
}

macro_rules! impl_datum {
    ($type:ty, $constr:ident) => {
        impl Datum for $type {
            fn into_wrapped(source: Self) -> Wrapped {
                Wrapped::$constr(source)
            }

            fn from_wrapped(wrapped: Wrapped) -> $type {
                if let Wrapped::$constr(v) = wrapped {
                    v
                } else {
                    panic!("Tried to get a {} from {:?}.", stringify!($ty), wrapped);
                }
            }
        }
    }
}

/// A wrapper for all the types of values that expressions can produce.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Wrapped {
    Int(isize),
    Dim(DimFact),
    Type(DataType),
}

impl_datum!(isize, Int);
impl_datum!(DimFact, Dim);
impl_datum!(DataType, Type);


/// An expression that can be compared by the solver.
pub trait Expression {
    type Output: Datum;

    /// Returns the current value of the expression in the given context.
    /// If the expression doesn't have a value, returns None.
    fn get(&self, context: &Context) -> Result<Option<Self::Output>>;

    /// Tries to set the value of the expression in the given context.
    fn set(&self, context: &mut Context, value: Self::Output) -> Result<()>;

    /// Returns the paths that the expression depends on.
    fn get_paths(&self) -> Vec<&Path>;
}


/// A constant expression (e.g. `2` or `DataType::DT_INT32`).
struct ConstantExpression<T: Datum>(T);

impl<T: Datum> Expression for ConstantExpression<T> {
    type Output = T;

    /// Returns the current value of the expression in the given context.
    fn get(&self, _: &Context) -> Result<Option<T>> {
        Ok(Some(self.0))
    }

    /// Tries to set the value of the expression in the given context.
    fn set(&self, _: &mut Context, value: T) -> Result<()> {
        if self.0 == value {
            Ok(())
        } else {
            bail!("Cannot set the value of constant {:?} to {:?}.", self.0, value);
        }
    }

    /// Returns the paths that the expression depends on.
    fn get_paths(&self) -> Vec<&Path> {
        vec![]
    }
}


/// A reference to a variable.
///
/// For instance, `inputs[0].rank` is a reference to the rank of the first
/// input. Internally, a reference holds a Vec<usize> called a path (see
/// the documentation for `Proxy::get_path`).
struct VariableExpression<T: Datum>(Path, PhantomData<T>);

impl<T: Datum> Expression for VariableExpression<T> {
    type Output = T;

    /// Returns the current value of the expression in the given context.
    fn get(&self, context: &Context) -> Result<Option<T>> {
        context.get(&self.0)
    }

    /// Tries to set the value of the expression in the given context.
    fn set(&self, context: &mut Context, value: T) -> Result<()> {
        context.set(&self.0, value)
    }

    /// Returns the paths that the expression depends on.
    fn get_paths(&self) -> Vec<&Path> {
        vec![&self.0]
    }
}


/// A scalar product between a constant and another expression.
struct ProductExpression<T, E>(T, E)
where
    T: Datum + Num + CheckedDiv,
    E: Expression<Output = T>;

impl<T, E> Expression for ProductExpression<T, E>
where
    T: Datum + Num + CheckedDiv,
    E: Expression<Output = T>
{
    type Output = T;

    /// Returns the current value of the expression in the given context.
    fn get(&self, context: &Context) -> Result<Option<T>> {
        let inner = self.1.get(context)?;

        Ok(inner.map(|v| v * self.0))
    }

    /// Tries to set the value of the expression in the given context.
    fn set(&self, context: &mut Context, value: T) -> Result<()> {
        let k = self.0;
        let m = value;

        if m == T::zero() && k == T::zero() {
            // We want to set 0 * x <- 0, so we don't have to do anything.
            Ok(())
        } else if m == T::zero() {
            // We want to set k * x <- 0, where k != 0, so we have to set x <- 0.
            self.1.set(context, T::zero())
        } else {
            // We want to set k * x <- m, where k and m != 0, so we will try
            // to set x <- m / k using a checked division. This way, if m is
            // not divisible by k, we will return Err instead of panicking.
            let div = m
                .checked_div(&k)
                .ok_or(format!(
                    "Cannot set the value of ({:?}, _) to {:?} because \
                    {:?} is not divisible by {:?}.", k, m, m, k))?;

            self.1.set(context, div)
        }
    }

    /// Returns the paths that the expression depends on.
    fn get_paths(&self) -> Vec<&Path> {
        self.1.get_paths()
    }
}


/// Converts &T to ConstantExpression<T>.
impl<'a, T> From<&'a T> for ConstantExpression<T> where T: Datum {
    fn from(c: &T) -> ConstantExpression<T> {
        ConstantExpression(*c)
    }
}

/// Converts T to ConstantExpression<T>.
impl<T> From<T> for ConstantExpression<T> where T: Datum {
    fn from(c: T) -> ConstantExpression<T> {
        ConstantExpression(c)
    }
}

/// Converts &IntProxy to VariableExpression<isize>.
impl<'a, T> From<&'a T> for VariableExpression<isize> where T: IntProxy {
    fn from(p: &T) -> VariableExpression<isize> {
        VariableExpression(p.get_path().to_vec(), PhantomData)
    }
}

/// Converts &DimProxy to VariableExpression<DimFact>.
impl<'a, T> From<&'a T> for VariableExpression<DimFact> where T: DimProxy {
    fn from(p: &T) -> VariableExpression<DimFact> {
        VariableExpression(p.get_path().to_vec(), PhantomData)
    }
}

/// Converts &TypeProxy to VariableExpression<DataType>.
impl<'a, T> From<&'a T> for VariableExpression<DataType> where T: TypeProxy {
    fn from(p: &T) -> VariableExpression<DataType> {
        VariableExpression(p.get_path().to_vec(), PhantomData)
    }
}

/// Converts (T, Into<Expression<Output = T>>) to ProductExpression<T>.
impl<T, E, I> From<(T, I)> for ProductExpression<T, E>
where
    T: Datum + Num + CheckedDiv,
    E: Expression<Output = T>,
    I: Into<E>,
{
    fn from((k, e): (T, I)) -> ProductExpression<T, E>
    {
        ProductExpression(k, e.into())
    }
}