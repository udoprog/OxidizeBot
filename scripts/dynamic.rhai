fn main(reg) {
    reg.register("dynamic", "handler");
}

fn handler(ctx) {
    ctx.respond("Goodbye You");
    let user = ctx.user();

    if user == () {
        print("Message caused by unknown user")
    } else {
        ctx.privmsg("Message for Everyone caused by " + user);
    }

    let count = db[["dynamic", user]];

    if count == () {
        count = 0;
    }

    db[["dynamic", user]] = 1;
}