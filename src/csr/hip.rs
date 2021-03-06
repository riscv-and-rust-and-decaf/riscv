use riscv_hypervisor_extension_proc_macro::generate_csr;
generate_csr!("Hip
1604
vssip,2,2,number,Software Interrupt
vstip,6,6,number,Timer Interrupt
vseip,10,10,number,External Interrupt 
sgeip,12,12,number,Guest External Interrupt 
end
Hypervisor Interrupt Pending Register.");