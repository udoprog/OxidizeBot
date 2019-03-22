use crate::{command, irc, utils};

/// Handler for the !8ball command.
pub struct EightBall {}

impl command::Handler for EightBall {
    fn handle<'m>(
        &mut self,
        _: command::Context<'_>,
        user: irc::User<'m>,
        it: &mut utils::Words<'m>,
    ) -> Result<(), failure::Error> {
        use rand::Rng as _;

        static MAGIC_8BALL_ANSWER: &[&'static str] = &[
            "It is certain.",
            "It is decidedly so.",
            "Without a doubt.",
            "Yes - definitely.",
            "You may rely on it.",
            "As I see it, yes.",
            "Most likely.",
            "Outlook good.",
            "Yes.",
            "Signs point to yes.",
            "Reply hazy, try again.",
            "Ask again later.",
            "Better not tell you now.",
            "Cannot predict now.",
            "Concentrate and ask again.",
            "Don't count on it.",
            "My reply is no.",
            "My sources say no.",
            "Outlook not so good.",
            "Very doubtful.",
        ];

        let rest = it.rest();

        if rest.trim().is_empty() {
            user.respond("Ask a question.");
            return Ok(());
        }

        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0, MAGIC_8BALL_ANSWER.len());

        if let Some(answer) = MAGIC_8BALL_ANSWER.get(index) {
            user.respond(answer);
        }

        Ok(())
    }
}
