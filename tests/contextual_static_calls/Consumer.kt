package consumers

import dep.Factory

fun text(): String? = Factory.empty()
fun nested(): String = Factory.describe(Factory.empty())
fun qualified(): CharSequence? = dep.Factory.empty()
fun shadow(Factory: dep.LocalFactory): String = Factory.empty()

class Assignments {
    var field: String? = "initial"
    fun run(java: dep.LocalFactory): Boolean {
        var local: String? = "initial"
        local = Factory.empty()
        field = Factory.empty()
        if (field != null || local != null) return false
        field = "again"
        this.field = Factory.empty()
        java.field = Factory.empty()
        return field == null && java.field == null
    }
}
