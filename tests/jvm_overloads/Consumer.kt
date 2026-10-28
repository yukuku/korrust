package overloads.consumer

import overloads.Member
import overloads.Singleton
import overloads.effects
import overloads.generic
import overloads.nullable
import overloads.top

fun result(): String {
    if (top(b = "B") != "1:B:3") return "top"
    if (Member().value() != "1:member") return "member"
    if (Member.staticValue() != "2:companion") return "companion"
    if (Member.instance() != 4L) return "companion default"
    if (Singleton.staticValue() != "5:object") return "object static"
    if (Singleton.instance() != 6L) return "object instance"
    if (generic("T") != "T") return "generic"
    if (nullable() != 2.5) return "nullable"
    return effects(required = "K")
}
