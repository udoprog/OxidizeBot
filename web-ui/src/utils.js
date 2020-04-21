const minute = 60,
    hour = minute * 60,
    day = hour * 24,
    week = day * 7;

/**
 * Get a human-readable duration since the given timestamp.
 */
export function humanDurationSince(from) {
    from = new Date(from);
    var delta = Math.round((+new Date - from) / 1000);

    var result;

    if (delta < 30) {
        result = 'Just now';
    } else if (delta < minute) {
        result = delta + ' seconds ago';
    } else if (delta < 2 * minute) {
        result = 'A minute ago'
    } else if (delta < hour) {
        result = Math.floor(delta / minute) + ' minutes ago';
    } else if (delta < hour * 2) {
        result = '1 hour ago'
    } else if (delta < day) {
        result = Math.floor(delta / hour) + ' hours ago';
    } else if (delta < day * 2) {
        result = 'Yesterday';
    } else if (delta < week) {
        result = Math.floor(delta / day) + ' days ago';
    } else if (delta < week * 2) {
        result = 'Last week';
    } else {
        result = Math.floor(delta / week) + ' weeks ago';
    }

    return result;
}