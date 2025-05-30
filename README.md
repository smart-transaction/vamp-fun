# vamp-fun

### Preparing the VM for vamp.fun

1.  Login to the VM as yourself, either via Cloud SSH or using its external IP address with your key.
    ```
    ssh -i .ssh/evgeny-stxn-cloud evgeny@34.69.26.172
    ```

1.  Create the deployer user.
    ```
    sudo adduser deployer
    ```
    The command `adduser` will ask for creating a password. You can create any of them, it won't be used.

1.  Configure passwordless sudo for deployer.
    *   Run the `sudo visudo`
    *   Add the following line after the `%sudo   ALL=(ALL:ALL) ALL`
        ```
        deployer ALL=(ALL) NOPASSWD:ALL
        ```
    *   Save changes

1.  Enable users for logging in as deployer.
    *   Add public SSH keys of all users you want to grant permission to into the file `/home/deployer/.ssh/authorized_keys`
    *   Set proper permissions
        ```
        sudo chown -R deployer:deployer /home/deployer/.ssh
        sudo chmod 700 /home/deployer/.ssh
        sudo chmod 600 /home/deployer/.ssh/authorized_keys
        ```

1.  Login to the VM as deployer
    ```
    ssh -i .ssh/evgeny-stxn-cloud deployer@34.69.26.172
    ```

1.  Configure the deployer for docker communications
    ```
    gcloud auth configure-docker us-central1-docker.pkg.dev
    ``` 
    That will create a configuration file .docker/config.json.

After that the VM is prepared for deploying vamp.fun