package consumer
import privateprobe.Vault
fun consume(): String = Vault().read() + ":" + Vault.Sibling().read()
