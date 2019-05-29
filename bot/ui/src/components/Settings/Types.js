import {String} from "./String";
import {Text} from "./Text";
import {Duration} from "./Duration";
import {Boolean} from "./Boolean";
import {Number} from "./Number";
import {Percentage} from "./Percentage";
import {Raw} from "./Raw";
import {Set} from "./Set";
import {Select} from "./Select";

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
    case "duration":
      return new Duration(type.optional);
    case "bool":
      return new Boolean(type.optional);
    case "string":
      return new String(type.optional);
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
    default:
      return new Raw(type.optional);
  }
}