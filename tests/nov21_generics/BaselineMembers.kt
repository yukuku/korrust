class Generic<C>(val seed: C) {
    fun <T> echo(value: T): T = value
}
interface GenericApi<C> { fun <T> echo(value: T): T }
