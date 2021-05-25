use core::fmt::{Display, Formatter, Debug};
use super::source::ErrorSource;
use super::tracer::ErrorMessageTracer;

pub struct ErrorReport<Detail, Trace> {
  pub detail: Detail,
  pub trace: Trace,
}

impl <Detail, Trace> ErrorSource<Trace> for ErrorReport<Detail, Trace> {
  type Source = Self;
  type Detail = Detail;

  fn error_details(source: Self::Source) -> (Self::Detail, Option<Trace>) {
    (source.detail, Some(source.trace))
  }
}

impl <Detail, Trace>
  ErrorReport<Detail, Trace>
{
  pub fn trace_from<E, Cont>
    ( source: E::Source,
      cont: Cont,
    ) -> Self
  where
    Detail: Display,
    E: ErrorSource<Trace>,
    Trace: ErrorMessageTracer,
    Cont: FnOnce(E::Detail) -> Detail,
  {
    let (detail1, m_trace1) = E::error_details(source);
    let detail2 = cont(detail1);
    match m_trace1 {
      Some(trace1) => {
        let trace2 = trace1.add_message(&detail2);
        ErrorReport { detail: detail2, trace: trace2 }
      }
      None => {
        let trace2 = Trace::new_message(&detail2);
        ErrorReport { detail: detail2, trace: trace2 }
      }
    }
  }
}

impl <Detail, Trace> Debug
  for ErrorReport<Detail, Trace>
where
  Trace: Debug
{
  fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
    self.trace.fmt(f)
  }
}

impl <Detail, Trace> Display
  for ErrorReport<Detail, Trace>
where
  Trace: Display
{
  fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
    self.trace.fmt(f)
  }
}

#[cfg(feature = "std")]
impl <Detail, Trace>
  std::error::Error
  for ErrorReport<Detail, Trace>
where
  Detail: Display,
  Trace: std::error::Error + 'static,
{
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    Some(&self.trace)
  }
}
