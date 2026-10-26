package compatibility.visibility

object ManglingCollision {
    internal fun token(): Int = 1
    fun `token$visibility_fixture`(): Int = 2
}
