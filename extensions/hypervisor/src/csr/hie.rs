use riscv_hypervisor_extension_proc_macro::generate_csr;
generate_csr!(
    "Hie
1540
vssie,2,2,number,Software Interrupt
vstie,6,6,number,Timer Interrupt
vseie,10,10,number,External Interrupt 
sgeie,12,12,number,Guest External Interrupt 
end
Hypervisor Interrupt Enable Register."
);
