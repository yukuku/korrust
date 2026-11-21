package datarefs

class Crate<T>(val value: T) {
    override fun toString(): String = "crate"
}

data class Selection(
    val ids: Set<Int>,
    val names: Map<String, String?>,
    val boxed: Crate<String>,
    val optional: Set<Int>? = null,
)
