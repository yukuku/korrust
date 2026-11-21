package novjvm.shadowalias
import novjvm.shadow.Throws as Declares
import novjvm.shadow.Synchronized as Monitor
@Declares("alias") @Monitor
fun plain(): Int = 25
