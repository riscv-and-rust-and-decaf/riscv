use riscv_hypervisor_extension_proc_macro::generate_csr;
generate_csr!(
    "Hstatus
1536
vsxl,33,32,VsxlValues,Vsxl32=1;Vsxl64;Vsxl128,Effective XLEN for VM.
vtsr,22,22,number,TSR for VM.
vtw,21,21,number,TW for VM.
vtvm,20,20,number,TVM for VM.
vgein,17,12,number,Virtual Guest External Interrupt Number.
hu,9,9,number,Hypervisor User mode.
spvp,8,8,number,Supervisor Previous Virtual Privilege.
gva,6,6,number,Guest Virtual Address.
vsbe,5,5,number,VS access endianness.
end
HStatus Register."
);
