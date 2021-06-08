pub use paste::paste;

/**

`define_error!` is the main macro that implements a mini DSL to
define error types using `flex-error`. The DSL syntax
is as follows:

```ignore
define_error! { ErrorName;
  SubErrorWithFieldsAndErrorSource
    { field1: Type1, field2: Type2, ... }
    [ ErrorSource ]
    | e | { format_args!(
      "format error message with field1: {}, field2: {}, source: {}",
      e.field1, e.field2, e.source)
    },
  SubErrorWithFieldsOnly
    { field1: Type1, field2: Type2, ... }
    | e | { format_args!(
      "format error message with field1: {}, field2: {}",
      e.field1, e.field2)
    },
  SubErrorWithSourceOnly
    [ ErrorSource ]
    | e | { format_args!(
      "format error message with source: {}",
      e.source)
    },
  SubError
    | e | { format_args!(
      "only suberror message")
    },
}
```

Behind the scene, `define_error!` does the following:

  - Define an enum with the postfix `Detail`, e.g. an error named
    `FooError` would have the enum `FooErrorDetail` defined.

  - Define the error name as a type alias to
    [`ErrorReport<ErrorNameDetail, DefaultTracer>`](crate::ErrorReport).
    e.g. `type FooError = ErrorReport<FooErrorDetail, DefaultTracer>;`.

  - For each suberror, does the following:

      - Define a variant with the suberror name in the detail enum.
        e.g. a `Bar` suberror in `FooError` becomes a `Bar`
        variant in `FooErrorDetail`.

      - Define a struct with the `Subdetail` postfix. e.g.
        `Bar` would have a `BarSubdetail` struct.

        - The struct contains all named fields if specified.

        - If an error source is specified, a `source` field is
          also defined with the type
          [`AsErrorDetail<ErrorSource>`](crate::AsErrorDetail).
          e.g. a suberror with
          [`DisplayError<SourceError>`](crate::DisplayError)
          would have the field `source: SourceError`.
          Because of this, the field name `source` is reserved and
          should not be present in other detail fields.

      - Implement [`Display`](std::fmt::Display) for the suberror
        using the provided formatter to format the arguments.
        The argument type of the formatter is the suberror subdetail struct.

      - Define a suberror constructor function in snake case with the postfix
        `_error`. e.g. `Bar` would have the constructor function `bar_error`.

        - The function accepts arguments according to the named fields specified.

        - If an error source is specified, the constructor function also accepts
          a last argument of type [`AsErrorSource<ErrorSource>`](crate::AsErrorSource).
          e.g. a suberror with [`DisplayError<SourceError>`](crate::DisplayError)
          would have the last argument of type `SourceError` in the constructor function.

        - The function returns the main error type. e.g. `FooError`, which is alias to
          [`ErrorReport<FooErrorDetail, DefaultTrace>`](crate::ErrorReport).

We can demonstrate the macro expansion of `define_error!` with the following example:

```ignore
// An external error type implementing Display
use external_crate::ExternalError;

define_error! { FooError;
  Bar
    { code: u32 }
    [ DisplayError<ExternalError> ]
    | e | { format_args!("Bar error with code {}", e.code) },
  Baz
    { extra: String }
    | e | { format_args!("General Baz error with extra detail: {}", e.extra) }
}
```

The above code will be expanded into something like follows:

```ignore
pub type FooError = Report<FooErrorDetail, DefaultTracer>;

#[derive(Debug)]
pub enum FooErrorDetail {
    Bar(BarSubdetail),
    Baz(BazSubdetail),
}

#[derive(Debug)]
pub struct BarSubdetail {
    pub code: u32,
    pub source: ExternalError
}

#[derive(Debug)]
pub struct BazSubdetail {
    pub extra: String
}

fn bar_error(code: u32, source: ExternalError) -> FooError { ... }
fn baz_error(extra: String) -> FooError { ... }

impl Display for BarSubdetail {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    let e = self;
    write!(f, "{}", format_args!("Bar error with code {}", e.code))
  }
}

impl Display for BazSubdetail {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    let e = self;
    write!(f, "{}", format_args!("General Baz error with extra detail: {}", e.code))
  }
}

impl Display for FooErrorDetail { ... }
```

For the detailed macro expansion, you can use [cargo-expand](https://github.com/dtolnay/cargo-expand)
to expand the Rust module that uses `define_error!` to see how the error definition
gets expanded.

Because `FooError` is defined as an alias to [`ErrorReport`](crate::ErrorReport),
it automatically implements [`ErrorSource`](crate::ErrorSource) and can be used
as a source in other error definitions. For example:

```ignore
define_error! { QuuxError;
  Foo
    { action: String }
    [ FooError ]
    | e | { format_args!("error arised from Foo when performing action {}", e.action) },
  ...
}
```

Would be expanded to include the following definitions:

```ignore
pub struct FooSubdetail {
  pub action: String,
  pub source: FooErrorDetail
}

pub fn foo_error(action: String, source: FooError) { ... }
```

In the formatter for `QuuxErrorDetail::Foo`, we can also see that it does not
need to include the error string from `FooError`. This is because the error
tracer already takes care of the source error trace, so the full trace is
automatically tracked inside `foo_error`. The outer error only need to
add additional detail about what caused the source error to be raised.

**/
#[macro_export]
macro_rules! define_error {
  ( $name:ident; $($expr:tt)+ ) => {
    $crate::define_error_with_tracer![
      $crate::DefaultTracer;
      [] $name;
      $( $expr )*
    ];
  };
  ( $derive:tt $name:ident; $($expr:tt)+ ) => {
    $crate::define_error_with_tracer![
      $crate::DefaultTracer;
      $derive $name;
      $( $expr )*
    ];
  };
}

/// This macro allows error types to be defined with custom error tracer types
/// other than [`DefaultTracer`](crate::DefaultTracer). Behind the scene,
/// a macro call to `define_error!{ ... } really expands to
/// `define_error_with_tracer!{ flex_error::DefaultTracer; ... }`
#[macro_export]
macro_rules! define_error_with_tracer {
  ( $tracer:ty;
    $derive:tt $name:ident;
    $(
      $suberror:ident
      $( { $( $arg_name:ident : $arg_type:ty ),* $(,)? } )?
      $( [ $source:ty ] )?
      | $formatter_arg:pat | $formatter:expr
    ),* $(,)?
  ) => {
    $crate::macros::paste![
      #[derive(Debug)]
      pub enum [< $name Detail >] {
        $(
          $suberror (
            [< $suberror Subdetail >]
          ),
        )*
      }

      $(
        $crate::define_suberror! {
          $tracer;
          $derive $name ;
          $suberror;
          ( $( $( $arg_name : $arg_type ),* )? )
          $( [ $source ] )?
        }

        impl core::fmt::Display for [< $suberror Subdetail >] {
          fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let $formatter_arg = self;
            write!(f, "{}",  $formatter)
          }
        }
      )*

      pub type $name = $crate::ErrorReport< [< $name Detail >], $tracer >;

      impl core::fmt::Display for [< $name Detail >] {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
          match self {
            $(
              Self::$suberror( suberror ) => {
                write!( f, "{}",  suberror )
              }
            ),*
          }
        }
      }

      $(
        $crate::define_error_constructor! {
          $tracer;
          $name;
          $suberror;
          ( $( $( $arg_name : $arg_type ),* )? )
          $( [ $source ] )?
        }
      )*
    ];
  };
}

/// Internal macro used to define suberror structs
#[macro_export]
#[doc(hidden)]
macro_rules! define_suberror {
  ( $tracer:ty;
    [ $( $attr:meta )? ] $name:ident;
    $suberror:ident;
    ( $( $arg_name:ident: $arg_type:ty ),* )
    $( [ $source:ty ] )?
  ) => {
    $crate::macros::paste! [
      $crate::define_struct![
        [ $( $attr )? ];
        pub struct [< $suberror Subdetail >] {
          $( pub $arg_name: $arg_type, )*
          $( pub source: $crate::AsErrorDetail<$source, $tracer> )?
        }
      ];
    ];
  };
}

#[macro_export]
#[doc(hidden)]
macro_rules! define_struct {
  ( [ ];
    $body:item
  ) => {
    $body
  };
  ( [ $attr:meta ];
    $body:item
  ) => {
    #[ $attr ]
    $body
  };
}

/// Internal macro used to define suberror constructor functions
#[macro_export]
#[doc(hidden)]
macro_rules! define_error_constructor {
  ( $tracer:ty;
    $name:ident;
    $suberror:ident;
    ( $( $arg_name:ident: $arg_type:ty ),* )
  ) => {
    $crate::macros::paste! [
      pub fn [< $suberror:snake _error >](
        $( $arg_name: $arg_type, )*
      ) -> $name
      {
        let detail = [< $name Detail >]::$suberror([< $suberror Subdetail >] {
          $( $arg_name, )*
        });

        let trace = < $tracer as $crate::ErrorMessageTracer >::new_message(&detail);
        $crate::ErrorReport {
          detail,
          trace,
        }
      }
    ];
  };
  ( $tracer:ty;
    $name:ident;
    $suberror:ident;
    ( $( $arg_name:ident: $arg_type:ty ),* )
    [ $source:ty ]
  ) => {
    $crate::macros::paste! [
      pub fn [< $suberror:snake _error >](
        $( $arg_name: $arg_type, )*
        source: $crate::AsErrorSource< $source, $tracer >
      ) -> $name
      {
        $crate::ErrorReport::trace_from::<$source, _>(source,
          | source_detail | {
            [< $name Detail >]::$suberror([< $suberror Subdetail >] {
              $( $arg_name, )*
              source: source_detail,
            })
          })
      }
    ];
  };
}
