/// Creates one or more new types that wrap a [`Uuid`].
///
/// This macro creates one or more `UUID` newtypes for which you can specify the name. The macro
/// generates code which implements many useful traits, including the ones needed to work with
/// [`diesel`].
///
/// The generated structs do not expose any method or trait to create an arbitrary instance[^See note], in
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
/// Keep in mind that there is no way[^See note] to construct an instance unless the instantiation is done
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
///
/// [^See note]: There is no way in normal usage to construct an instance. The exception is with the
/// `dangerous_new` method, which is gated behind the `dangerous-construction` feature and intended
/// for special cases (including testing). If the `dangerous-construction` feature is enabled, it is
/// recommended to use [`clippy::disallowed_methods`](https://rust-lang.github.io/rust-clippy/stable/index.html#disallowed_methods) to prevent the usage of `dangerous_new` outside
/// of the desired situations.
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
                $crate::__private::std::fmt::Debug,
                $crate::__private::std::clone::Clone,
                $crate::__private::std::marker::Copy,
                $crate::__private::std::cmp::PartialEq,
                $crate::__private::std::cmp::Eq,
                $crate::__private::std::cmp::PartialOrd,
                $crate::__private::std::cmp::Ord,
                $crate::__private::std::hash::Hash,
            )]
            $vis struct $name($crate::__private::uuid::Uuid);

            impl $name {
                $crate::__typed_uuid__impl_dangerous_construction!($vis);

                /// Gets the actual `Uuid`.
                #[must_use]
                #[allow(unused)]
                $vis fn get(&self) -> $crate::__private::uuid::Uuid {
                    self.0
                }
            }

            impl $crate::__private::diesel::deserialize::FromSql<$crate::__private::diesel::pg::sql_types::Uuid, $crate::__private::diesel::pg::Pg> for $name {
                fn from_sql(value: $crate::__private::diesel::pg::PgValue<'_>) -> $crate::__private::diesel::deserialize::Result<Self> {
                    $crate::__private::uuid::Uuid::from_slice(value.as_bytes())
                        .map(Self)
                        .map_err(Into::into)
                }
            }

            impl $crate::__private::diesel::serialize::ToSql<$crate::__private::diesel::pg::sql_types::Uuid, $crate::__private::diesel::pg::Pg> for $name {
                fn to_sql<'b>(&'b self, out: &mut $crate::__private::diesel::serialize::Output<'b, '_, $crate::__private::diesel::pg::Pg>) -> $crate::__private::diesel::serialize::Result {
                    $crate::__private::std::io::Write::write_all(out, self.0.as_bytes())
                        .map(|_| $crate::__private::diesel::serialize::IsNull::No)
                        .map_err(Into::into)
                }
            }

            // These are manually implemented because the derive macro uses `diesel` instead of the
            // private path.
            impl<
                '__expr,
            > $crate::__private::diesel::expression::AsExpression<$crate::__private::diesel::pg::sql_types::Uuid>
            for &'__expr $name {
                type Expression = $crate::__private::diesel::internal::derives::as_expression::Bound<
                    $crate::__private::diesel::pg::sql_types::Uuid,
                    Self,
                >;
                fn as_expression(
                    self,
                ) -> <Self as $crate::__private::diesel::expression::AsExpression<
                    $crate::__private::diesel::pg::sql_types::Uuid,
                >>::Expression {
                    $crate::__private::diesel::internal::derives::as_expression::Bound::new(self)
                }
            }
            impl<
                '__expr,
            > $crate::__private::diesel::expression::AsExpression<
                $crate::__private::diesel::sql_types::Nullable<$crate::__private::diesel::pg::sql_types::Uuid>,
            > for &'__expr $name {
                type Expression = $crate::__private::diesel::internal::derives::as_expression::Bound<
                    $crate::__private::diesel::sql_types::Nullable<
                        $crate::__private::diesel::pg::sql_types::Uuid,
                    >,
                    Self,
                >;
                fn as_expression(
                    self,
                ) -> <Self as $crate::__private::diesel::expression::AsExpression<
                    $crate::__private::diesel::sql_types::Nullable<
                        $crate::__private::diesel::pg::sql_types::Uuid,
                    >,
                >>::Expression {
                    $crate::__private::diesel::internal::derives::as_expression::Bound::new(self)
                }
            }
            impl<
                '__expr,
                '__expr2,
            > $crate::__private::diesel::expression::AsExpression<$crate::__private::diesel::pg::sql_types::Uuid>
            for &'__expr2 &'__expr $name {
                type Expression = $crate::__private::diesel::internal::derives::as_expression::Bound<
                    $crate::__private::diesel::pg::sql_types::Uuid,
                    Self,
                >;
                fn as_expression(
                    self,
                ) -> <Self as $crate::__private::diesel::expression::AsExpression<
                    $crate::__private::diesel::pg::sql_types::Uuid,
                >>::Expression {
                    $crate::__private::diesel::internal::derives::as_expression::Bound::new(self)
                }
            }
            impl<
                '__expr,
                '__expr2,
            > $crate::__private::diesel::expression::AsExpression<
                $crate::__private::diesel::sql_types::Nullable<$crate::__private::diesel::pg::sql_types::Uuid>,
            > for &'__expr2 &'__expr $name {
                type Expression = $crate::__private::diesel::internal::derives::as_expression::Bound<
                    $crate::__private::diesel::sql_types::Nullable<
                        $crate::__private::diesel::pg::sql_types::Uuid,
                    >,
                    Self,
                >;
                fn as_expression(
                    self,
                ) -> <Self as $crate::__private::diesel::expression::AsExpression<
                    $crate::__private::diesel::sql_types::Nullable<
                        $crate::__private::diesel::pg::sql_types::Uuid,
                    >,
                >>::Expression {
                    $crate::__private::diesel::internal::derives::as_expression::Bound::new(self)
                }
            }
            impl<
                __DB,
            > $crate::__private::diesel::serialize::ToSql<
                $crate::__private::diesel::sql_types::Nullable<$crate::__private::diesel::pg::sql_types::Uuid>,
                __DB,
            > for $name
            where
                __DB: $crate::__private::diesel::backend::Backend,
                Self: $crate::__private::diesel::serialize::ToSql<
                    $crate::__private::diesel::pg::sql_types::Uuid,
                    __DB,
                >,
            {
                fn to_sql<'__b>(
                    &'__b self,
                    out: &mut $crate::__private::diesel::serialize::Output<'__b, '_, __DB>,
                ) -> $crate::__private::diesel::serialize::Result {
                    $crate::__private::diesel::serialize::ToSql::<
                        $crate::__private::diesel::pg::sql_types::Uuid,
                        __DB,
                    >::to_sql(self, out)
                }
            }
            impl $crate::__private::diesel::expression::AsExpression<
                $crate::__private::diesel::pg::sql_types::Uuid,
            > for $name {
                type Expression = $crate::__private::diesel::internal::derives::as_expression::Bound<
                    $crate::__private::diesel::pg::sql_types::Uuid,
                    Self,
                >;
                fn as_expression(
                    self,
                ) -> <Self as $crate::__private::diesel::expression::AsExpression<
                    $crate::__private::diesel::pg::sql_types::Uuid,
                >>::Expression {
                    $crate::__private::diesel::internal::derives::as_expression::Bound::new(self)
                }
            }
            impl $crate::__private::diesel::expression::AsExpression<
                $crate::__private::diesel::sql_types::Nullable<$crate::__private::diesel::pg::sql_types::Uuid>,
            > for $name {
                type Expression = $crate::__private::diesel::internal::derives::as_expression::Bound<
                    $crate::__private::diesel::sql_types::Nullable<
                        $crate::__private::diesel::pg::sql_types::Uuid,
                    >,
                    Self,
                >;
                fn as_expression(
                    self,
                ) -> <Self as $crate::__private::diesel::expression::AsExpression<
                    $crate::__private::diesel::sql_types::Nullable<
                        $crate::__private::diesel::pg::sql_types::Uuid,
                    >,
                >>::Expression {
                    $crate::__private::diesel::internal::derives::as_expression::Bound::new(self)
                }
            }

            impl<__DB, __ST> $crate::__private::diesel::deserialize::Queryable<__ST, __DB> for $name
            where
                __DB: $crate::__private::diesel::backend::Backend,
                __ST: $crate::__private::diesel::sql_types::SingleValue,
                Self: $crate::__private::diesel::deserialize::FromSql<__ST, __DB>,
            {
                type Row = Self;
                fn build(row: Self) -> $crate::__private::diesel::deserialize::Result<Self> {
                    Ok(row)
                }
            }

            impl $crate::__private::std::cmp::PartialEq<$crate::__private::uuid::Uuid> for $name {
                fn eq(&self, other: &$crate::__private::uuid::Uuid) -> bool {
                    self.0 == *other
                }
            }

            impl $crate::__private::std::cmp::PartialEq<$name> for $crate::__private::uuid::Uuid {
                fn eq(&self, other: &$name) -> bool {
                    *self == other.0
                }
            }

            impl $crate::__private::std::cmp::PartialEq<$crate::__private::uuid::NonNilUuid> for $name {
                fn eq(&self, other: &$crate::__private::uuid::NonNilUuid) -> bool {
                    self.0 == *other
                }
            }

            impl $crate::__private::std::cmp::PartialEq<$name> for $crate::__private::uuid::NonNilUuid {
                fn eq(&self, other: &$name) -> bool {
                    *self == other.0
                }
            }

            impl $crate::__private::std::cmp::PartialOrd<$crate::__private::uuid::Uuid> for $name {
                fn partial_cmp(&self, other: &$crate::__private::uuid::Uuid) -> $crate::__private::std::option::Option<$crate::__private::std::cmp::Ordering> {
                    $crate::__private::std::cmp::PartialOrd::partial_cmp(&self.0, other)
                }
            }

            impl $crate::__private::std::cmp::PartialOrd<$name> for $crate::__private::uuid::Uuid {
                fn partial_cmp(&self, other: &$name) -> $crate::__private::std::option::Option<$crate::__private::std::cmp::Ordering> {
                    $crate::__private::std::cmp::PartialOrd::partial_cmp(self, &other.0)
                }
            }

            impl $crate::__private::std::convert::AsRef<[u8]> for $name {
                fn as_ref(&self) -> &[u8] {
                    $crate::__private::std::convert::AsRef::as_ref(&self.0)
                }
            }

            impl $crate::__private::std::convert::AsRef<$crate::__private::uuid::Uuid> for $name {
                fn as_ref(&self) -> &$crate::__private::uuid::Uuid {
                    &self.0
                }
            }

            impl $crate::__private::std::borrow::Borrow<$crate::__private::uuid::Uuid> for $name {
                fn borrow(&self) -> &$crate::__private::uuid::Uuid {
                    &self.0
                }
            }

            impl $crate::__private::std::convert::From<$name> for $crate::__private::uuid::Uuid {
                fn from(value: $name) -> Self {
                    value.0
                }
            }

            $crate::__typed_uuid__forward_from!(
                $name:
                $crate::__private::uuid::fmt::Braced,
                $crate::__private::uuid::fmt::Hyphenated,
                $crate::__private::uuid::fmt::Simple,
                $crate::__private::uuid::fmt::Urn,
                $crate::__private::std::string::String,
                $crate::__private::std::vec::Vec<u8>,
            );

            impl $crate::__private::std::fmt::Display for $name {
                fn fmt(&self, f: &mut $crate::__private::std::fmt::Formatter<'_>) -> $crate::__private::std::fmt::Result {
                    $crate::__private::std::fmt::Display::fmt(&self.0, f)
                }
            }

            impl $crate::__private::std::fmt::LowerHex for $name {
                fn fmt(&self, f: &mut $crate::__private::std::fmt::Formatter<'_>) -> $crate::__private::std::fmt::Result {
                    $crate::__private::std::fmt::LowerHex::fmt(&self.0, f)
                }
            }

            impl $crate::__private::std::fmt::UpperHex for $name {
                fn fmt(&self, f: &mut $crate::__private::std::fmt::Formatter<'_>) -> $crate::__private::std::fmt::Result {
                    $crate::__private::std::fmt::UpperHex::fmt(&self.0, f)
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
            impl $crate::__private::std::convert::From<$name> for $ty {
                fn from(value: $name) -> Self {
                    Self::from(value.0)
                }
            }
        )+
    };
}

#[macro_export]
#[doc(hidden)]
#[cfg(feature = "dangerous-construction")]
macro_rules! __typed_uuid__impl_dangerous_construction {
    ($vis:vis) => {
        /// Creates a new typed `Uuid` which does not come from the database.
        #[must_use]
        #[allow(unused)]
        $vis fn dangerous_new(inner: $crate::__private::uuid::Uuid) -> Self {
            Self(inner)
        }
    };
}

#[macro_export]
#[doc(hidden)]
#[cfg(not(feature = "dangerous-construction"))]
macro_rules! __typed_uuid__impl_dangerous_construction {
    ($vis:vis) => {};
}

#[macro_export]
#[doc(hidden)]
#[cfg(feature = "serde")]
macro_rules! __typed_uuid__impl_serde {
    ($name:ident) => {
        impl $crate::__private::serde::Serialize for $name {
            fn serialize<S>(
                &self,
                serializer: S,
            ) -> $crate::__private::std::result::Result<S::Ok, S::Error>
            where
                S: $crate::__private::serde::Serializer,
            {
                $crate::__private::serde::Serialize::serialize(&self.0, serializer)
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

#[cfg(test)]
mod test {
    use uuid::Uuid;

    #[test]
    fn creates_new_typed_uuid() {
        crate::typed_uuid!(pub FooId);
        let inner = Uuid::new_v4();
        let new = FooId::dangerous_new(inner);
        assert_eq!(new.get(), inner);
    }
}
