use futures::future::{Either, Map, FutureExt, ok, Ready};
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};

pub struct Conditional<T> {
    middle: T,
    active: bool,
}

impl<T> Conditional<T> {
    pub fn new(middle: T, active: bool) -> Self {
        Self {middle, active}
    }
}

impl<T, S, B> Transform<S, ServiceRequest> for Conditional<T>
where
    T: Transform<S, ServiceRequest, Response = S::Response, Error = S::Error>,
    T::Transform: 'static,
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = T::InitError;
    type Transform = ConditionalMiddleware<T::Transform, S>;
    type Future = Either<
        Map<T::Future, fn(Result<T::Transform, T::InitError>) -> Result<Self::Transform, Self::InitError>>,
        Ready<Result<Self::Transform, Self::InitError>>,
    >;

    fn new_transform(&self, service: S) -> Self::Future {
        if self.active {
            let f = self.middle
                .new_transform(service)
                .map(|trans: Result<T::Transform, T::InitError>| -> Result<Self::Transform, Self::InitError> {
                    match trans {
                        Ok(t) => Ok(ConditionalMiddleware::Enable(t)),
                        Err(err) => Err(err),
                    }
                } as fn(Result<T::Transform, T::InitError>) -> Result<Self::Transform, Self::InitError>);
            Either::Left(f)
        } else {
            Either::Right(ok(ConditionalMiddleware::Disable(service)))
        }
    }
}

pub enum ConditionalMiddleware<E, D> {
    Enable(E),
    Disable(D),
}

impl<E, D, B> Service<ServiceRequest> for ConditionalMiddleware<E, D>
where
    E: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    E::Future: 'static,
    D: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    D::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Either<E::Future, D::Future>;

    fn poll_ready(&self, ctx: &mut core::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        use ConditionalMiddleware as CM;
        match self {
            CM::Enable(service) => service.poll_ready(ctx),
            CM::Disable(service) => service.poll_ready(ctx),
        }
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        use ConditionalMiddleware as CM;
        match self {
            CM::Enable(service) => Either::Left(service.call(req)),
            CM::Disable(service) => Either::Right(service.call(req)),
        }
    }
}
