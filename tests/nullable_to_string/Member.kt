package membershadow
import stringify.Probe
fun Probe?.toString(): String = "extension"
fun member(value: Probe): String = value.toString()
fun nullableMember(value: Probe?): String = value.toString()
