// Copyright (c) Sean Lawlor
//
// This source code is licensed under both the MIT license found in the
// LICENSE-MIT file in the root directory of this source tree.

//! Serialization definitions for `ractor_cluster` over-the-network message encoding. This
//! contains helpful types for encoding and decoding messages from raw byte vectors
//!
//! We implement the trait automatically for 8, 16, 32, 64, and 128 bit numerics but we specifically
//! DO NOT implement it for arch-specific types [isize] or [usize] because the may encode at one size
//! on one host and decode at the wrong size at the other host.
//!
//! We additionally provide implementations for [String], [Vec<`char`>], and [Vec<_>] of all numeric values.

/// Trait for use with `ractor_cluster_derive::RactorClusterMessage`
/// derive macro. It defines argument and reply message types which
/// are serializable to/from byte payloads so code can be autogenerated
/// for you by macros for over-the-wire message formats between actors
pub trait BytesConvertable {
    /// Serialize this type to a vector of bytes. Panics are acceptable
    fn into_bytes(self) -> Vec<u8>;
    /// Deserialize this type from a vector of bytes. Panics are acceptable
    fn from_bytes(bytes: Vec<u8>) -> Self;
}

#[cfg(feature = "blanket_serde")]
/// Contains a blanket implementation for all types that implement serde::Serialize and serde::Deserialize
mod impls {
    use crate::BytesConvertable;

    impl<T: serde::Serialize + serde::de::DeserializeOwned> BytesConvertable for T {
        fn from_bytes(bytes: Vec<u8>) -> Self {
            pot::from_slice(&bytes).unwrap()
        }
        fn into_bytes(self) -> Vec<u8> {
            vec![]
        }
    }
}

#[cfg(not(feature = "blanket_serde"))]
/// Contains the default implementations for the `BytesConvertable` trait
mod impls {
    use crate::BytesConvertable;

    // ==================== Primitive implementations ==================== //

    macro_rules! implement_numeric {
        {$ty: ty} => {
            impl BytesConvertable for $ty {
                fn into_bytes(self) -> Vec<u8> {
                    self.to_be_bytes().to_vec()
                }
                fn from_bytes(bytes: Vec<u8>) -> Self {
                    let mut data = [0u8; std::mem::size_of::<Self>()];
                    data.copy_from_slice(&bytes[..std::mem::size_of::<Self>()]);
                    Self::from_be_bytes(data)
                }
            }
        };
    }

    implement_numeric! {i8}
    implement_numeric! {i16}
    implement_numeric! {i32}
    implement_numeric! {i64}
    implement_numeric! {i128}

    implement_numeric! {u8}
    implement_numeric! {u16}
    implement_numeric! {u32}
    implement_numeric! {u64}
    implement_numeric! {u128}

    implement_numeric! {f32}
    implement_numeric! {f64}

    impl BytesConvertable for () {
        fn into_bytes(self) -> Vec<u8> {
            Vec::new()
        }
        fn from_bytes(_: Vec<u8>) -> Self {}
    }

    impl BytesConvertable for bool {
        fn into_bytes(self) -> Vec<u8> {
            if self {
                vec![1u8]
            } else {
                vec![0u8]
            }
        }
        fn from_bytes(bytes: Vec<u8>) -> Self {
            bytes[0] == 1u8
        }
    }

    impl BytesConvertable for char {
        fn into_bytes(self) -> Vec<u8> {
            let u = self as u32;
            u.into_bytes()
        }
        fn from_bytes(bytes: Vec<u8>) -> Self {
            let u = u32::from_bytes(bytes);
            Self::from_u32(u).unwrap()
        }
    }

    impl BytesConvertable for String {
        fn into_bytes(self) -> Vec<u8> {
            self.into_bytes()
        }
        fn from_bytes(bytes: Vec<u8>) -> Self {
            String::from_utf8(bytes).unwrap()
        }
    }

    // ==================== Vectorized implementations ==================== //

    macro_rules! implement_vectorized_numeric {
        {$ty: ty} => {
            impl BytesConvertable for Vec<$ty> {
                fn into_bytes(self) -> Vec<u8> {
                    let mut result = vec![0u8; self.len() * std::mem::size_of::<$ty>()];
                    for (offset, item) in self.into_iter().enumerate() {
                        result[offset * std::mem::size_of::<$ty>() .. offset * std::mem::size_of::<$ty>() + std::mem::size_of::<$ty>()].copy_from_slice(&item.to_be_bytes());
                    }
                    result
                }
                fn from_bytes(bytes: Vec<u8>) -> Self {
                    let num_el = bytes.len() / std::mem::size_of::<$ty>();
                    let mut result = vec![<$ty>::MIN; num_el];

                    let mut data = [0u8; std::mem::size_of::<$ty>()];
                    for offset in 0..num_el {
                        data.copy_from_slice(&bytes[offset * std::mem::size_of::<$ty>() .. offset * std::mem::size_of::<$ty>() + std::mem::size_of::<$ty>()]);
                        result[offset] = <$ty>::from_be_bytes(data);
                    }

                    result
                }
            }
        };
    }

    implement_vectorized_numeric! {i8}
    implement_vectorized_numeric! {i16}
    implement_vectorized_numeric! {i32}
    implement_vectorized_numeric! {i64}
    implement_vectorized_numeric! {i128}

    // We explicitly skip u8, as it has a more
    // optimized definition
    impl BytesConvertable for Vec<u8> {
        fn into_bytes(self) -> Vec<u8> {
            self
        }
        fn from_bytes(bytes: Vec<u8>) -> Self {
            bytes
        }
    }
    implement_vectorized_numeric! {u16}
    implement_vectorized_numeric! {u32}
    implement_vectorized_numeric! {u64}
    implement_vectorized_numeric! {u128}

    implement_vectorized_numeric! {f32}
    implement_vectorized_numeric! {f64}

    impl BytesConvertable for Vec<bool> {
        fn into_bytes(self) -> Vec<u8> {
            let mut result = vec![0u8; self.len()];
            for (ptr, item) in self.into_iter().enumerate() {
                let byte = if item { [1u8] } else { [0u8] };
                result[ptr..ptr + 1].copy_from_slice(&byte);
            }
            result
        }
        fn from_bytes(bytes: Vec<u8>) -> Self {
            let num_el = bytes.len();
            let mut result = vec![false; num_el];
            for (ptr, byte) in bytes.into_iter().enumerate() {
                result[ptr] = byte == 1u8;
            }

            result
        }
    }

    impl BytesConvertable for Vec<char> {
        fn into_bytes(self) -> Vec<u8> {
            let data = self.into_iter().map(|c| c as u32).collect::<Vec<_>>();
            data.into_bytes()
        }
        fn from_bytes(bytes: Vec<u8>) -> Self {
            let u32s = <Vec<u32>>::from_bytes(bytes);
            u32s.into_iter()
                .map(|u| char::from_u32(u).unwrap())
                .collect::<Vec<_>>()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BytesConvertable;
    use crate::{message::BoxedDowncastErr, Message};
    use rand::{distributions::Alphanumeric, thread_rng, Rng};

    fn random_string() -> String {
        thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect()
    }

    macro_rules! run_basic_type_test {
        {$ty: ty} => {
            paste::item! {
                #[test]
                fn [< test_bytes_conversion_ $ty >] () {
                    let test_data: $ty = rand::thread_rng().gen();
                    let bytes = test_data.clone().into_bytes();
                    let back = <$ty as BytesConvertable>::from_bytes(bytes);
                    assert_eq!(test_data, back);

                    let serialized_message = test_data.serialize().expect("Failed to serialize type");
                    let deserialized_message = <$ty as Message>::deserialize(serialized_message).expect("Failed to deserialize type");
                    assert_eq!(test_data, deserialized_message);
                }
            }
        };
    }

    macro_rules! run_vector_type_test {
        {$ty: ty} => {
            paste::item! {
                #[test]
                fn [< test_bytes_conversion_vec_ $ty >] () {
                    let mut rng = rand::thread_rng();
                    let num_pts: usize = rng.gen_range(10..50);
                    let test_data = (0..num_pts).into_iter().map(|_| rng.gen()).collect::<Vec<$ty>>();

                    let bytes = test_data.clone().into_bytes();
                    let back = <Vec<$ty> as BytesConvertable>::from_bytes(bytes);

                    assert_eq!(test_data, back);

                    let serialized_message = test_data.clone().serialize().expect("Failed to serialize type");
                    let deserialized_message = <Vec<$ty> as Message>::deserialize(serialized_message).expect("Failed to deserialize type");
                    assert_eq!(test_data, deserialized_message);
                }
            }
        };
    }

    run_basic_type_test! {i8}
    run_basic_type_test! {i16}
    run_basic_type_test! {i32}
    run_basic_type_test! {i64}
    run_basic_type_test! {i128}
    run_basic_type_test! {u8}
    run_basic_type_test! {u16}
    run_basic_type_test! {u32}
    run_basic_type_test! {u64}
    run_basic_type_test! {u128}
    run_basic_type_test! {f32}
    run_basic_type_test! {f64}
    run_basic_type_test! {char}
    run_basic_type_test! {bool}

    #[test]
    #[allow(non_snake_case)]
    fn test_bytes_conversion_String() {
        let test_data: String = random_string();
        let bytes = <String as BytesConvertable>::into_bytes(test_data.clone());
        let back = <String as BytesConvertable>::from_bytes(bytes);
        assert_eq!(test_data, back);
    }

    run_vector_type_test! {i8}
    run_vector_type_test! {i16}
    run_vector_type_test! {i32}
    run_vector_type_test! {i64}
    run_vector_type_test! {i128}
    run_vector_type_test! {u8}
    run_vector_type_test! {u16}
    run_vector_type_test! {u32}
    run_vector_type_test! {u64}
    run_vector_type_test! {u128}
    run_vector_type_test! {f32}
    run_vector_type_test! {f64}
    run_vector_type_test! {char}
    run_vector_type_test! {bool}

    #[test]
    fn test_boxed_downcast_error() {
        let err = BoxedDowncastErr;
        println!("{err}");
        println!("{err:?}");
    }
}
