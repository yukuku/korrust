package visibilityqualified
import java.lang.StringBuilder as Builder
import java.util.UUID as Token
fun keep(value: java.lang.StringBuilder): java.lang.StringBuilder = value
class Outer {
 class Entry(val value: Int)
 fun keep(value: Entry): Entry = value
}

fun keepToken(value: java.util.UUID): java.util.UUID = value
