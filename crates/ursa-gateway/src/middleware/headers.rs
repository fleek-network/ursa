use hyper::body::HttpBody;
use hyper::{Request, Response, StatusCode};
use std::str::FromStr;
use tower_http::validate_request::ValidateRequest;

#[derive(Clone)]
pub struct Cid(String);

impl Cid {
    pub fn to_string(self) -> String {
        self.0
    }
}

#[derive(Clone)]
pub struct GetRequestCid;

impl<B> ValidateRequest<B> for GetRequestCid
where
    B: HttpBody + Default,
{
    type ResponseBody = B;

    fn validate(&mut self, request: &mut Request<B>) -> Result<(), Response<Self::ResponseBody>> {
        match request.headers().get(hyper::header::HOST) {
            None => {
                tracing::trace!("missing Host header");
                return Err(bad_request::<B>());
            }
            Some(host) => {
                let host = host.to_str().map_err(|_| bad_request::<B>())?;
                let (cid, _) = host.split_once('.').ok_or_else(|| bad_request::<B>())?;
                tracing::trace!("extracted cid {cid:?} from header");
                libipld::Cid::from_str(cid).map_err(|e| {
                    tracing::error!("invalid cid: {e:?}");
                    bad_request::<B>()
                })?;
                let cid = cid.to_string();
                request.extensions_mut().insert(Cid(cid));
                Ok(())
            }
        }
    }
}

fn bad_request<B>() -> Response<B>
where
    B: HttpBody + Default,
{
    let mut response = Response::new(B::default());
    *response.status_mut() = StatusCode::BAD_REQUEST;
    response
}
