-- AET-WO-4406 adversarial: a ROMFS script, run at boot because SCR_ENABLE
-- defaults to 1, that moves SERIAL3 off GPS after var_info's default applied.
param:set_default('SERIAL3_PROTOCOL', -1)
param:set('SERIAL3_PROTOCOL', -1)
