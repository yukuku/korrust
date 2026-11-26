package storage

fun read(): Int = Holder.first.id
fun store(item: Holder?): Holder? {
    Holder.last = item
    return Holder.Companion.last
}
fun named(): String = Named.value
