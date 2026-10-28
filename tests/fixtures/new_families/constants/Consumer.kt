package newfamilies.constants.consumer

import newfamilies.constants.*

fun check(): String {
    if (BASE != 42 || DECLARED_LONG != 3L || INT_OVERFLOW_TO_LONG != -2147483648L) return "numbers"
    if (TEXT != "ab25" || SOURCE_IMPORT != 14 || IMPORTED_VALUE != 21) return "references"
    if (Holder.TOTAL != 47 || Holder.Nested.NEXT != 48) return "objects"
    if (returnedText() != "ab25" || annotated() != 45) return "calls"
    return "OK"
}
