use pinnacle_api_defs::pinnacle::{
    tag::v1::{
        self, AddRequest, AddResponse, GetActiveRequest, GetActiveResponse, GetNameRequest,
        GetNameResponse, GetOutputNameRequest, GetOutputNameResponse, GetRequest, GetResponse,
        MoveToOutputRequest, MoveToOutputResponse, RemoveRequest, SetActiveRequest,
        SwitchToRequest,
    },
    util::v1::SetOrToggle,
};
use tonic::{Request, Status};

use crate::{
    api::{TonicResult, run_unary, run_unary_no_response},
    output::OutputName,
    state::WithState,
    tag::TagId,
};

#[tonic::async_trait]
impl v1::tag_service_server::TagService for super::TagService {
    async fn get(&self, _request: Request<GetRequest>) -> TonicResult<GetResponse> {
        run_unary(&self.sender, move |state| {
            let tags = state.pinnacle.outputs.iter().flat_map(|op| {
                op.with_state(|state| {
                    state
                        .tags
                        .iter()
                        .filter(|tag| !tag.defunct())
                        .cloned()
                        .collect::<Vec<_>>()
                })
            });

            let tag_ids = tags.map(|tag| tag.id().to_inner()).collect();

            Ok(GetResponse { tag_ids })
        })
        .await
    }

    async fn get_active(
        &self,
        request: Request<GetActiveRequest>,
    ) -> TonicResult<GetActiveResponse> {
        let tag_id = TagId::new(request.into_inner().tag_id);
        run_unary(&self.sender, move |state| {
            let active = tag_id
                .tag(&state.pinnacle)
                .map(|tag| tag.active())
                .unwrap_or_default();

            Ok(GetActiveResponse { active })
        })
        .await
    }

    async fn get_name(&self, request: Request<GetNameRequest>) -> TonicResult<GetNameResponse> {
        let tag_id = TagId::new(request.into_inner().tag_id);
        run_unary(&self.sender, move |state| {
            let name = tag_id
                .tag(&state.pinnacle)
                .map(|tag| tag.name())
                .unwrap_or_default();

            Ok(GetNameResponse { name })
        })
        .await
    }

    async fn get_output_name(
        &self,
        request: Request<GetOutputNameRequest>,
    ) -> TonicResult<GetOutputNameResponse> {
        let tag_id = TagId::new(request.into_inner().tag_id);
        run_unary(&self.sender, move |state| {
            let output_name = tag_id
                .tag(&state.pinnacle)
                .and_then(|tag| Some(tag.output(&state.pinnacle)?.name()))
                .unwrap_or_default();

            Ok(GetOutputNameResponse { output_name })
        })
        .await
    }

    async fn set_active(&self, request: Request<SetActiveRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let tag_id = TagId::new(request.tag_id);

        let set_or_toggle = request.set_or_toggle();

        if set_or_toggle == SetOrToggle::Unspecified {
            return Err(Status::invalid_argument("unspecified set or toggle"));
        }

        let active = match set_or_toggle {
            SetOrToggle::Set => Some(true),
            SetOrToggle::Unset => Some(false),
            SetOrToggle::Toggle => None,
            SetOrToggle::Unspecified => unreachable!(),
        };

        run_unary_no_response(&self.sender, move |state| {
            let Some(tag) = tag_id.tag(&state.pinnacle) else {
                return;
            };

            crate::api::tag::set_active(state, &tag, active);
        })
        .await
    }

    async fn switch_to(&self, request: Request<SwitchToRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let tag_id = TagId::new(request.tag_id);

        run_unary_no_response(&self.sender, move |state| {
            let Some(tag) = tag_id.tag(&state.pinnacle) else { return };
            crate::api::tag::switch_to(state, &tag);
        })
        .await
    }

    async fn add(&self, request: Request<AddRequest>) -> TonicResult<AddResponse> {
        let request = request.into_inner();

        let output_name = OutputName(request.output_name);

        let tag_names = request.tag_names;

        run_unary(&self.sender, move |state| {
            let tags = crate::api::tag::add(state, tag_names, output_name);

            let tag_ids = tags.into_iter().map(|tag| tag.id().to_inner()).collect();

            Ok(AddResponse { tag_ids })
        })
        .await
    }

    async fn move_to_output(
        &self,
        request: Request<MoveToOutputRequest>,
    ) -> TonicResult<MoveToOutputResponse> {
        let request = request.into_inner();

        let output_name = OutputName(request.output_name);

        let tag_ids = request.tag_ids.into_iter().map(TagId::new);

        run_unary(&self.sender, move |state| {
            let tags_to_move = tag_ids
                .flat_map(|id| id.tag(&state.pinnacle))
                .collect::<Vec<_>>();

            use crate::api::tag::TagMoveToOutputError;
            use pinnacle_api_defs::pinnacle::tag::v1::{
                MoveToOutputSameWindowOnTwoOutputs, move_to_output_response,
            };

            let kind = match crate::api::tag::move_to_output(state, tags_to_move, output_name) {
                Ok(()) => None,
                Err(TagMoveToOutputError::OutputDoesNotExist(output_name)) => Some(
                    move_to_output_response::Kind::OutputDoesNotExist(output_name.0),
                ),
                Err(TagMoveToOutputError::SameWindowOnTwoOutputs(tags)) => {
                    Some(move_to_output_response::Kind::SameWindowOnTwoOutputs(
                        MoveToOutputSameWindowOnTwoOutputs {
                            tag_ids: tags.into_iter().map(|tag| tag.id().to_inner()).collect(),
                        },
                    ))
                }
            };
            Ok(MoveToOutputResponse { kind })
        })
        .await
    }

    async fn remove(&self, request: Request<RemoveRequest>) -> TonicResult<()> {
        let request = request.into_inner();

        let tag_ids = request.tag_ids.into_iter().map(TagId::new);

        run_unary_no_response(&self.sender, move |state| {
            let tags_to_remove = tag_ids
                .flat_map(|id| id.tag(&state.pinnacle))
                .collect::<Vec<_>>();

            crate::api::tag::remove(state, tags_to_remove);
        })
        .await
    }
}
