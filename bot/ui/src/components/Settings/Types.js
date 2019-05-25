import {StringType} from "./String";
import {DurationType} from "./Duration";
import {BooleanType} from "./Boolean";
import {NumberType} from "./Number";
import {PercentageType} from "./Percentage";
import {RawType} from "./Raw";
import {SetType} from "./Set";

/**
 * Decode the given type and value.
 *
 * @param {object} type the type to decode
 * @param {any} value the value to decode
 */
export function decode(type) {
  if (type === null) {
    return RawType;
  }

  switch (type.id) {
    case "duration":
      return new DurationType(type.optional);
    case "bool":
      return new BooleanType(type.optional);
    case "string":
      return new StringType(type.optional);
    case "number":
      return new NumberType(type.optional);
    case "percentage":
      return new PercentageType(type.optional);
    case "set":
      let value = decode(type.value);
      return new SetType(type.optional, value);
    default:
      return new RawType(type.optional);
  }
}