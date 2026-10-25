package next.probes
class InfixBox(val value: Int) {
    infix fun combine(other: Int): Int = value + other
}
