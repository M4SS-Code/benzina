/// Creates one or more new types that wrap a [`Uuid`].
///
/// This macro creates one or more `UUID` newtypes for which you can specify the name. The macro
/// generates code which implements many useful traits, including the ones needed to work with
/// [`diesel`].
///
/// The generated structs do not expose any method or trait to create an arbitrary instance, in
/// order to provide the guarantee that the `UUID` is valid. However, it is possible to choose to
/// add traits and methods to customize the behavior.
///
#[cfg_attr(
    not(feature = "example-generated"),
    doc = "To see the documentation of a generated typed `UUID`, consider re-building the \
        documentation with the feature `example-generated` enabled."
)]
#[cfg_attr(
    feature = "example-generated",
    doc = "You can see the documentation for an example generated `UUID` struct \
        [`here`](crate::example_generated::FooId)."
)]
///
/// [`Uuid`]: ::uuid::Uuid
///
/// # Examples
///
/// The usage of the macro is pretty straightforward: you specify the name of the type, and it is
/// also possible to add attributes as needed.
///
/// ```
/// use benzina::typed_uuid;
/// use uuid::Uuid;
///
/// typed_uuid! (
///     /// You can add documentation.
///     FooId,
///
///     // Attributes and visibility work
///     #[expect(dead_code)]
///     #[derive(Default)]
///     pub BarId,
/// );
///
/// fn use_foo(foo_id: FooId) {
///     // Implements and derives many traits like `Display`
///     println!("{foo_id}");
///
///     let _actual_uuid: Uuid = foo_id.get();
/// }
/// ```
///
/// Keep in mind that there is no way to construct an instance unless the instantiation is done
/// inside the module containing the new type or a submodule of it:
///
/// ```compile_fail,E0603
/// use uuid::Uuid;
///
/// mod inner {
///     benzina::typed_uuid!(pub Foo);
/// }
///
/// // Error E0603: a constructor is private if any of the fields is private
/// let foo = inner::Foo(Uuid::default());
/// //               ^^^ private tuple struct constructor
/// ```
#[macro_export]
macro_rules! typed_uuid {
    (
        $(
            $(#[$attr:meta])*
            $vis:vis $name:ident
        ),+ $(,)?
    ) => {
        $(
            $(#[$attr])*
            #[derive(
                ::std::fmt::Debug,
                ::std::clone::Clone,
                ::std::marker::Copy,
                ::std::cmp::PartialEq,
                ::std::cmp::Eq,
                ::std::cmp::PartialOrd,
                ::std::cmp::Ord,
                ::std::hash::Hash,
                ::diesel::expression::AsExpression,
                ::diesel::deserialize::FromSqlRow,
            )]
            #[diesel(sql_type = ::diesel::pg::sql_types::Uuid)]
            $vis struct $name(::uuid::Uuid);

            impl $name {
                /// Gets the actual [`Uuid`].
                ///
                /// [`Uuid`]: ::uuid::Uuid
                #[must_use]
                pub fn get(&self) -> ::uuid::Uuid {
                    self.0
                }
            }

            impl ::diesel::deserialize::FromSql<::diesel::pg::sql_types::Uuid, ::diesel::pg::Pg> for $name {
                fn from_sql(value: ::diesel::pg::PgValue<'_>) -> ::diesel::deserialize::Result<Self> {
                    uuid::Uuid::from_slice(value.as_bytes())
                        .map(Self)
                        .map_err(Into::into)
                }
            }

            impl ::diesel::serialize::ToSql<::diesel::pg::sql_types::Uuid, ::diesel::pg::Pg> for $name {
                fn to_sql<'b>(&'b self, out: &mut ::diesel::serialize::Output<'b, '_, ::diesel::pg::Pg>) -> ::diesel::serialize::Result {
                    ::std::io::Write::write_all(out, self.0.as_bytes())
                        .map(|_| ::diesel::serialize::IsNull::No)
                        .map_err(Into::into)
                }
            }

            impl ::std::cmp::PartialEq<::uuid::Uuid> for $name {
                fn eq(&self, other: &::uuid::Uuid) -> bool {
                    self.0 == *other
                }
            }

            impl ::std::cmp::PartialEq<$name> for ::uuid::Uuid {
                fn eq(&self, other: &$name) -> bool {
                    *self == other.0
                }
            }

            impl ::std::cmp::PartialEq<::uuid::NonNilUuid> for $name {
                fn eq(&self, other: &::uuid::NonNilUuid) -> bool {
                    self.0 == *other
                }
            }

            impl ::std::cmp::PartialEq<$name> for ::uuid::NonNilUuid {
                fn eq(&self, other: &$name) -> bool {
                    *self == other.0
                }
            }

            impl ::std::convert::AsRef<[u8]> for $name {
                fn as_ref(&self) -> &[u8] {
                    ::std::convert::AsRef::as_ref(&self.0)
                }
            }

            impl ::std::convert::AsRef<::uuid::Uuid> for $name {
                fn as_ref(&self) -> &::uuid::Uuid {
                    &self.0
                }
            }

            impl ::std::convert::From<$name> for ::uuid::Uuid {
                fn from(value: $name) -> Self {
                    value.0
                }
            }

            $crate::__typed_uuid__forward_from!(
                $name:
                ::uuid::fmt::Braced,
                ::uuid::fmt::Hyphenated,
                ::uuid::fmt::Simple,
                ::uuid::fmt::Urn,
                ::std::string::String,
                ::std::vec::Vec<u8>,
            );

            impl ::std::fmt::Display for $name {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    ::std::fmt::Display::fmt(&self.0, f)
                }
            }

            impl ::std::fmt::LowerHex for $name {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    ::std::fmt::LowerHex::fmt(&self.0, f)
                }
            }

            impl ::std::fmt::UpperHex for $name {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    ::std::fmt::UpperHex::fmt(&self.0, f)
                }
            }

            $crate::__typed_uuid__impl_serde!($name);
            )+
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __typed_uuid__forward_from {
    ($name:ident: $($ty:path),+ $(,)?) => {
        $(
            impl ::std::convert::From<$name> for $ty {
                fn from(value: $name) -> Self {
                    Self::from(value.0)
                }
            }
        )+
    };
}

#[macro_export]
#[doc(hidden)]
#[cfg(feature = "serde")]
macro_rules! __typed_uuid__impl_serde {
    ($name:ident) => {
        impl ::serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                ::serde::Serialize::serialize(&self.0, serializer)
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
#[cfg(not(feature = "serde"))]
macro_rules! __typed_uuid__impl_serde {
    ($name:ident) => {};
}
