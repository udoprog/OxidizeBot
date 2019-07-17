use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "gql/twitch.graphql",
    query_path = "gql/query_badges.graphql",
    response_derives = "Debug, Clone, Serialize"
)]
pub struct Badges;
