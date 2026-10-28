package visibilitygeneric

class Dual<A, B>(initial: A) {
    private var value: A = initial
    fun readAsB(other: Dual<B, B>): B = other.value
}
