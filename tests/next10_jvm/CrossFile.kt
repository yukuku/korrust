package nextjvm.cross

import nextjvm.api.answer
import nextjvm.api.ModuleOnly
import nextjvm.api.Secret

fun crossFile(): Int = answer() + ModuleOnly(3).value + Secret.create(4).value
