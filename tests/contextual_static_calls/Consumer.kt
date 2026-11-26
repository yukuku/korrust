package consumers

import dep.Factory

fun text(): String? = Factory.empty()
fun nested(): String = Factory.describe(Factory.empty())
fun qualified(): CharSequence? = dep.Factory.empty()
fun shadow(Factory: dep.LocalFactory): String = Factory.empty()
