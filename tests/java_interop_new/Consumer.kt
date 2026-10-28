package newinterop.consumer

import newinterop.Bean
import newinterop.Facade
import newinterop.GenericBean
import newinterop.use.generic
import newinterop.use.hiddenParent
import newinterop.use.nullable
import newinterop.use.objectMembers
import newinterop.use.staticCalls

fun result(): Boolean = staticCalls().startsWith("i2sai3:6:17:") && objectMembers(Bean()) && generic(GenericBean("O")) == "OK" && nullable(null) == null && hiddenParent(Facade()).startsWith("hidden:visible:")
