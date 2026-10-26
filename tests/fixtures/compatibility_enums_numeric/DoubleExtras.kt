package compatibility.enumsnumeric

const val constant: Double = 1.25
fun leading(): Double = .5
fun sourceNan(): Double = Double.NaN
fun sourceInfinity(): Double = Double.POSITIVE_INFINITY
fun explicitJavaNan(): Double = java.lang.Double.NaN
fun defaulted(value: Double = 1.5): Double = value
class DefaultBox(val value: Double = 2.5)
fun defaultProbe(): Double = defaulted() + DefaultBox().value
fun capture(seed: Double): () -> Double = { seed + 0.25 }
fun counter(seed: Double): () -> Double { var value = seed; return { value += 0.5; value } }
fun stringValue(value: Double): String = value.toString()
