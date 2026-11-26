import localshadow.local
import importedshadow.imported
import aliasshadow.aliased
fun consume(): String = nullable(null) + local(null) + imported(null) + aliased(null)
