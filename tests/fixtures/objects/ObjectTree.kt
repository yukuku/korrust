package korrust.probes.objects

object ObjectTree {
    const val ROOT = 1
    object Left {
        const val KEY = "left"
        object Leaf { const val VALUE = 11 }
    }
    object Right { const val KEY = "right" }
    object Empty
}
