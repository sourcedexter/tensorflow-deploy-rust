use std::collections::HashMap;
use std::marker::PhantomData;

use analyser::interface::*;
use ops::prelude::*;
use tensor::Datum;
use Result;

#[derive(Debug, Clone, Default, new)]
pub struct Pack<T: Datum> {
    n: usize, // The number of inputs
    axis: usize,
    _phantom: PhantomData<T>,
}

pub fn pack(pb: &::tfpb::node_def::NodeDef) -> Result<Box<Op>> {
    let dtype = pb.get_attr_datatype("T")?;
    let n = pb.get_input().len();
    let axis = pb.get_attr_int("axis")?;

    Ok(boxed_new!(Pack(dtype)(n, axis)))
}

impl<T> Op for Pack<T>
where
    T: Datum,
{
    /// Evaluates the operation given the input tensors.
    fn eval(&self, inputs: Vec<TensorView>) -> Result<Vec<TensorView>> {
        use ndarray::Axis;
        let views = inputs
            .iter()
            .map(|m| Ok(T::tensor_to_view(&*m)?.insert_axis(Axis(self.axis))))
            .collect::<Result<Vec<_>>>()?;
        let array = ::ndarray::stack(Axis(self.axis), &*views)?;
        Ok(vec![T::array_into_tensor(array).into()])
    }

    /// Returns the attributes of the operation and their values.
    fn get_attributes(&self) -> HashMap<&'static str, Attr> {
        hashmap!{
            "T"    => Attr::DataType(T::datatype()),
            "n"    => Attr::Usize(self.n),
            "axis" => Attr::Usize(self.axis),
        }
    }
}

impl<T: Datum> InferenceRulesOp for Pack<T> {
    fn rules<'r, 'p: 'r, 's: 'r>(
        &'s self,
        solver: &mut Solver<'r>,
        inputs: &'p TensorsProxy,
        outputs: &'p TensorsProxy,
    ) {
        let output = &outputs[0];
        let n = self.n;
        let axis = self.axis;
        solver
            .equals(&inputs.len, n as isize)
            .equals(&outputs.len, 1)
            .equals_all((0..n).map(|i| bexp(&inputs[i].rank)).collect())
            .equals_zero(wrap!((-1, &output.rank), (1isize, 1), (1, &inputs[0].rank)))
            .given(&inputs[0].rank, move |solver, r: usize| {
                (0..r).for_each(|d| {
                    solver.equals_all((0..n).map(|i| bexp(&inputs[i].shape[d])).collect());
                })
            })
            .given(&inputs[0].rank, move |solver, r: usize| {
                (0..axis).for_each(|d| {
                    solver.equals(&output.shape[d], &inputs[0].shape[d]);
                });
                if r > 0 {
                    (axis..(r - 1)).for_each(|d| {
                        solver.equals(&output.shape[d + 1], &inputs[0].shape[d]);
                    });
                }
            })
            .equals(&output.shape[axis], n as isize);
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]
    use super::*;
    use ndarray::arr2;
    use Tensor;

    #[test]
    fn pack_0() {
        let inputs = vec![
            Tensor::i32s(&[2], &[1, 4]).unwrap().into(),
            Tensor::i32s(&[2], &[2, 5]).unwrap().into(),
            Tensor::i32s(&[2], &[3, 6]).unwrap().into(),
        ];
        assert_eq!(
            Pack::<i32>::new(3, 0)
                .eval(inputs.clone())
                .unwrap()
                .remove(0)
                .into_tensor(),
            Tensor::from(arr2(&[[1, 4], [2, 5], [3, 6]]))
        );
        assert_eq!(
            Pack::<i32>::new(3, 1)
                .eval(inputs.clone())
                .unwrap()
                .remove(0)
                .into_tensor(),
            Tensor::from(arr2(&[[1, 2, 3], [4, 5, 6]]))
        );
    }

    #[test]
    fn pack_1() {
        let pack = Pack::<i32>::new(3, 0);
        let input = Tensor::i32s(&[0], &[]).unwrap();
        let exp: Tensor = Tensor::i32s(&[1, 0], &[]).unwrap();
        let found = pack.eval(vec![input.into()]).unwrap();

        assert!(
            exp.close_enough(&found[0]),
            "expected: {:?} found: {:?}",
            exp,
            found[0]
        )
    }
}
