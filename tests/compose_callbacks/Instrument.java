import java.nio.file.*;
import org.jetbrains.org.objectweb.asm.*;
public final class Instrument {
    public static void main(String[] arguments) throws Exception {
        Path file=Path.of(arguments[0]);
        ClassReader reader=new ClassReader(Files.readAllBytes(file));
        ClassWriter writer=new ClassWriter(ClassWriter.COMPUTE_MAXS);
        reader.accept(new ClassVisitor(Opcodes.ASM9,writer) {
            public MethodVisitor visitMethod(int access,String name,String descriptor,String signature,String[] exceptions) {
                return new MethodVisitor(Opcodes.ASM9,super.visitMethod(access,name,descriptor,signature,exceptions)) {
                    boolean composable;
                    public AnnotationVisitor visitAnnotation(String descriptor,boolean visible) {
                        composable |= descriptor.equals("Landroidx/compose/runtime/Composable;");
                        return super.visitAnnotation(descriptor,visible);
                    }
                    public void visitCode() {
                        super.visitCode();
                        if(!composable) return;
                        Type[] types=Type.getArgumentTypes(descriptor);
                        int composer=-1,slot=(access&Opcodes.ACC_STATIC)!=0?0:1;
                        int[] slots=new int[types.length];
                        for(int i=0;i<types.length;i++) {
                            slots[i]=slot; slot+=types[i].getSize();
                            if(types[i].getDescriptor().equals("Landroidx/compose/runtime/Composer;")) composer=i;
                        }
                        if(composer<0) throw new AssertionError(descriptor);
                        super.visitLdcInsn(name);
                        super.visitVarInsn(Opcodes.ALOAD,slots[composer]);
                        int count=types.length-composer-1;
                        super.visitLdcInsn(count);
                        super.visitIntInsn(Opcodes.NEWARRAY,Opcodes.T_INT);
                        for(int i=0;i<count;i++) {
                            if(!types[composer+1+i].equals(Type.INT_TYPE)) throw new AssertionError(descriptor);
                            super.visitInsn(Opcodes.DUP);
                            super.visitLdcInsn(i);
                            super.visitVarInsn(Opcodes.ILOAD,slots[composer+1+i]);
                            super.visitInsn(Opcodes.IASTORE);
                        }
                        super.visitMethodInsn(Opcodes.INVOKESTATIC,"composeprobe/Capture","record","(Ljava/lang/String;Landroidx/compose/runtime/Composer;[I)V",false);
                    }
                };
            }
        },0);
        Files.write(file,writer.toByteArray());
    }
}
