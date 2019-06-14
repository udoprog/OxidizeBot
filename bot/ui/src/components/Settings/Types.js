import {String} from "./String";
import {Text} from "./Text";
import {Duration} from "./Duration";
import {Boolean} from "./Boolean";
import {Number} from "./Number";
import {Percentage} from "./Percentage";
import {Raw} from "./Raw";
import {Set} from "./Set";
import {Select} from "./Select";
import {Typeahead} from "./Typeahead";
import {Oauth2Config} from "./Oauth2Config";
import * as format from "./Format";
import * as timezones from "timezones.json";

/**
 * Decode the given type and value.
 *
 * @param {object} type the type to decode
 * @param {any} value the value to decode
 */
export function decode(type) {
  if (type === null) {
    throw new Error(`bad type: ${type}`);
  }

  let value = null;

  switch (type.id) {
    case "oauth2-config":
      return new Oauth2Config(type.optional);
    case "duration":
      return new Duration(type.optional);
    case "bool":
      return new Boolean(type.optional);
    case "string":
      return new String(type.optional, format.decode(type.format), type.placeholder);
    case "text":
      return new Text(type.optional);
    case "number":
      return new Number(type.optional);
    case "percentage":
      return new Percentage(type.optional);
    case "set":
      value = decode(type.value);
      return new Set(type.optional, value);
    case "select":
      value = decode(type.value);
      return new Select(type.optional, value, type.options);
    case "time-zone":
      value = new String(false, new format.None(), "");;
      return new Typeahead(type.optional, value, timezoneOptions, "timezone");
    default:
      return new Raw(type.optional);
  }
}

const timezoneOptions = buildTimezoneOptions();

function buildTimezoneOptions() {
  return timezones.flatMap(p => p.utc.map(tz => {
    let name = tz;
    let n = name.indexOf('/');

    if (n > 0) {
      name = name.substring(n + 1);
    } else {
      name = name;
    }

    name = name.replace('_', ' ');

    let id = null;

    if (p.offset >= 0) {
      id = `UTC+${tzOffset(p.offset)}`;
    } else {
      id = `UTC-${tzOffset(-p.offset)}`;
    }

    return {
      title: `${id} - ${name} (${p.abbr})`,
      value: tz,
    }
  }));
}

function tzOffset(offset) {
  let rest = offset % 1;
  return `${pad(offset - rest, 2)}${pad(60 * rest, 2)}`;

  function pad(num, size) {
    var s = num + "";

    while (s.length < size) {
      s = "0" + s;
    }

    return s;
  }
}