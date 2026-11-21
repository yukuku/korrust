package wrapperproof;

import android.app.Application;
import android.content.Context;
import android.database.DatabaseUtils;
import android.database.sqlite.SQLiteDatabase;
import kotlin.Unit;
import org.junit.Test;
import org.junit.runner.RunWith;
import org.robolectric.RobolectricTestRunner;
import org.robolectric.RuntimeEnvironment;
import org.robolectric.annotation.Config;
import org.robolectric.annotation.SQLiteMode;
import yuku.alkitab.base.storage.InternalDb;
import yuku.alkitab.base.storage.InternalDbHelper;
import yuku.alkitab.base.storage.InternalDbTxWrapper;

@RunWith(RobolectricTestRunner.class)
@Config(sdk = 28, application = Application.class, manifest = Config.NONE)
@SQLiteMode(SQLiteMode.Mode.NATIVE)
public final class DatabaseSmoke {
    private static void rows(SQLiteDatabase db, long expected) {
        long actual = DatabaseUtils.longForQuery(db, "SELECT COUNT(*) FROM compiler_transaction_proof", null);
        if (actual != expected) throw new AssertionError("rows " + actual + " != " + expected);
        if (db.inTransaction()) throw new AssertionError("transaction leaked");
    }

    @Test
    public void originalDatabaseCommitsRollsBackAndPreservesCallbackExceptions() {
        Context context = RuntimeEnvironment.getApplication();
        yuku.afw.App.initWithAppContext(context);
        context.deleteDatabase("AlkitabDb");
        InternalDbHelper helper = new InternalDbHelper(context);
        try {
            InternalDb internal = new InternalDb(helper);
            SQLiteDatabase db = helper.getWritableDatabase();
            if (DatabaseUtils.longForQuery(db, "SELECT COUNT(*) FROM ProgressMark", null) != 5) throw new AssertionError("original schema initialization");
            db.execSQL("CREATE TABLE compiler_transaction_proof(value INTEGER NOT NULL)");
            InternalDbTxWrapper.INSTANCE.transact(internal, handle -> {
                db.execSQL("INSERT INTO compiler_transaction_proof VALUES (1)");
                return Unit.INSTANCE;
            });
            rows(db, 0);
            InternalDbTxWrapper.INSTANCE.transact(internal, handle -> {
                db.execSQL("INSERT INTO compiler_transaction_proof VALUES (2)");
                handle.commit();
                return Unit.INSTANCE;
            });
            rows(db, 1);
            InternalDbTxWrapper.INSTANCE.transact(internal, handle -> {
                db.execSQL("INSERT INTO compiler_transaction_proof VALUES (3)");
                handle.commit();
                handle.commit();
                return Unit.INSTANCE;
            });
            rows(db, 2);
            RuntimeException marker = new RuntimeException("callback marker");
            try {
                InternalDbTxWrapper.INSTANCE.transact(internal, handle -> {
                    db.execSQL("INSERT INTO compiler_transaction_proof VALUES (4)");
                    throw marker;
                });
                throw new AssertionError("callback exception lost");
            } catch (RuntimeException error) {
                if (error != marker) throw new AssertionError("callback exception changed", error);
            }
            rows(db, 2);
            try {
                InternalDbTxWrapper.INSTANCE.transact(internal, handle -> {
                    db.execSQL("INSERT INTO compiler_transaction_proof VALUES (5)");
                    handle.commit();
                    throw marker;
                });
                throw new AssertionError("callback exception lost after commit");
            } catch (RuntimeException error) {
                if (error != marker) throw new AssertionError("callback exception changed after commit", error);
            }
            rows(db, 2);
            InternalDbTxWrapper.TxHandle[] saved = new InternalDbTxWrapper.TxHandle[1];
            InternalDbTxWrapper.INSTANCE.transact(internal, handle -> {
                db.execSQL("INSERT INTO compiler_transaction_proof VALUES (6)");
                saved[0] = handle;
                return Unit.INSTANCE;
            });
            rows(db, 2);
            saved[0].commit();
            rows(db, 2);
            if (DatabaseUtils.longForQuery(db, "SELECT SUM(value) FROM compiler_transaction_proof", null) != 5) throw new AssertionError("committed data");
            System.out.println("TRANSACTION_PROOF: rollback, commit, repeated commit, exception, exception after commit, retained handle");
        } finally {
            helper.close();
            context.deleteDatabase("AlkitabDb");
        }
    }
}
