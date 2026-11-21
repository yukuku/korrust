package classifierprobe

interface Observer {
    fun changed(mode: Catalog.Mode)
}

class Holder(val mode: Catalog.Mode)

object Lookup {
    @JvmStatic
    fun <C> value(entry: Catalog.Entry<C>): String = entry.key

    @JvmStatic
    fun <C> find(entries: List<Catalog.Entry<C>>, key: String): Catalog.Entry<C>? {
        for (entry in entries) {
            if (entry.key == key) return entry
        }
        return null
    }

    @JvmStatic
    fun marker(value: Any): Boolean = value is Marker

    @JvmStatic
    fun mode(value: Any): Boolean = value is Catalog.Mode

    @JvmStatic
    fun kotlinNested(value: Any): Boolean = value is KotlinOuter.Nested
}
