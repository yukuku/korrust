package wrapperproof
import yuku.alkitab.base.storage.InternalDb
import yuku.alkitab.base.storage.InternalDbTxWrapper
fun transact(internal: InternalDb) {
    InternalDbTxWrapper.transact(internal) { it.commit() }
}
fun handle(action: () -> Unit): InternalDbTxWrapper.TxHandle =
    InternalDbTxWrapper.TxHandle { action() }
